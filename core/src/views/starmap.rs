use anyhow::{anyhow, Result};
use ndarray::{Array2, ArrayView1};
use umap_rs::{GraphParams, Umap, UmapConfig};
use uuid::Uuid;

use crate::{
    db::umap::{UmapCache, UmapPointRecord},
    dto::StarmapPointDTO,
    error::ServiceError,
    models::note::{Note, NoteReader},
};

const DEFAULT_UMAP_NEIGHBORS: usize = 15;
const DEFAULT_UMAP_EPOCHS: usize = 60;

pub(crate) struct StarmapView<'a, 'b> {
    tx: &'a redb::ReadTransaction,
    reader: &'a NoteReader<'b>,
}

impl<'a, 'b> StarmapView<'a, 'b> {
    pub fn new(tx: &'a redb::ReadTransaction, reader: &'a NoteReader<'b>) -> Self {
        Self { tx, reader }
    }

    pub fn points(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        self.cached_points()
    }

    pub fn cached_points(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        UmapCache::iter(self.tx)?
            .map(|res| {
                res.map(|(note_id, point)| StarmapPointDTO {
                    id: Uuid::from_bytes(note_id).to_string(),
                    x: point.x,
                    y: point.y,
                })
                .map_err(redb::Error::from)
                .map_err(ServiceError::from)
            })
            .collect()
    }

    pub fn collect_live_vectors(&self) -> Result<Vec<(Uuid, Vec<f32>)>, ServiceError> {
        let vector_store = Note::vector_index();
        let mut entries = Vec::new();

        for item in vector_store.iter(self.tx)? {
            let (key_guard, vector) = item.map_err(redb::Error::from)?;
            let note_id = Uuid::from_bytes(key_guard.value());
            let Some(note_ref) = self.reader.get_ref_by_id(&note_id)? else {
                continue;
            };

            if note_ref.is_deleted()
                || self
                    .reader
                    .has_next_version(&note_id)
                    .map_err(redb::Error::from)?
            {
                continue;
            }

            entries.push((note_id, vector));
        }

        Ok(entries)
    }

    pub fn build_projection(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        let entries = self.collect_live_vectors()?;
        let vectors = entries
            .iter()
            .map(|(_, vector)| vector.clone())
            .collect::<Vec<_>>();
        let coordinates = project_vectors_to_2d(&vectors)?;

        Ok(entries
            .into_iter()
            .zip(coordinates)
            .map(|((note_id, _), [x, y])| StarmapPointDTO {
                id: note_id.to_string(),
                x,
                y,
            })
            .collect())
    }

    pub fn rebuild_cache(
        &self,
        tx: &redb::WriteTransaction,
    ) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        let points = self.build_projection()?;
        UmapCache::clear(tx)?;

        for point in &points {
            let note_id = Uuid::parse_str(&point.id)?;
            UmapCache::put(
                tx,
                &note_id.into_bytes(),
                &UmapPointRecord {
                    x: point.x,
                    y: point.y,
                },
            )?;
        }

        Ok(points)
    }

    pub fn clear_cache(tx: &redb::WriteTransaction) -> Result<usize, ServiceError> {
        UmapCache::clear(tx).map_err(Into::into)
    }

    pub fn refresh_for_note_change(
        &self,
        tx: &redb::WriteTransaction,
        _note_id: Uuid,
    ) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        self.rebuild_cache(tx)
    }
}

fn project_vectors_to_2d(vectors: &[Vec<f32>]) -> Result<Vec<[f32; 2]>> {
    match vectors.len() {
        0 => return Ok(Vec::new()),
        1 => return Ok(vec![[0.0, 0.0]]),
        2 => return Ok(vec![[-1.0, 0.0], [1.0, 0.0]]),
        _ => {}
    }

    let feature_dim = vectors[0].len();
    if feature_dim == 0 {
        return Err(anyhow!("cannot project empty vectors"));
    }

    if vectors.iter().any(|vector| vector.len() != feature_dim) {
        return Err(anyhow!("all vectors must share the same dimension"));
    }

    let data = Array2::from_shape_vec(
        (vectors.len(), feature_dim),
        vectors
            .iter()
            .flat_map(|vector| vector.iter().copied())
            .collect(),
    )?;

    let n_neighbors = DEFAULT_UMAP_NEIGHBORS.min(vectors.len().saturating_sub(1));
    let (knn_indices, knn_dists) = build_bruteforce_knn(vectors, n_neighbors);
    let init = build_initial_layout(&data);

    let mut config = UmapConfig {
        n_components: 2,
        ..Default::default()
    };
    config.graph = GraphParams {
        n_neighbors,
        ..Default::default()
    };
    config.optimization.n_epochs = Some(DEFAULT_UMAP_EPOCHS);

    let embedding = Umap::new(config)
        .fit(
            data.view(),
            knn_indices.view(),
            knn_dists.view(),
            init.view(),
        )
        .into_embedding();

    let mut projected = embedding
        .outer_iter()
        .map(|row: ArrayView1<'_, f32>| {
            let x = row.get(0).copied().unwrap_or_default();
            let y = row.get(1).copied().unwrap_or_default();
            [
                if x.is_finite() { x } else { 0.0 },
                if y.is_finite() { y } else { 0.0 },
            ]
        })
        .collect::<Vec<_>>();

    normalize_points(&mut projected);
    Ok(projected)
}

fn build_bruteforce_knn(vectors: &[Vec<f32>], n_neighbors: usize) -> (Array2<u32>, Array2<f32>) {
    let n_samples = vectors.len();
    let mut knn_indices = Array2::<u32>::zeros((n_samples, n_neighbors));
    let mut knn_dists = Array2::<f32>::zeros((n_samples, n_neighbors));

    for (i, vector) in vectors.iter().enumerate() {
        let mut distances = vectors
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, other)| (j as u32, euclidean_distance(vector, other)))
            .collect::<Vec<_>>();

        distances.sort_by(|a, b| a.1.total_cmp(&b.1));

        for (neighbor_idx, (id, dist)) in distances.into_iter().take(n_neighbors).enumerate() {
            knn_indices[(i, neighbor_idx)] = id;
            knn_dists[(i, neighbor_idx)] = dist;
        }
    }

    (knn_indices, knn_dists)
}

fn build_initial_layout(data: &Array2<f32>) -> Array2<f32> {
    let n_samples = data.shape()[0];
    let n_features = data.shape()[1];
    let mut init = Array2::<f32>::zeros((n_samples, 2));

    for i in 0..n_samples {
        init[(i, 0)] = data[(i, 0)];
        init[(i, 1)] = if n_features > 1 {
            data[(i, 1)]
        } else {
            data[(i, 0)] * 0.5
        };
    }

    normalize_array2_columns(&mut init);
    init
}

fn normalize_array2_columns(data: &mut Array2<f32>) {
    for axis in 0..data.shape()[1] {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;

        for row in 0..data.shape()[0] {
            let value = data[(row, axis)];
            min = min.min(value);
            max = max.max(value);
        }

        let span = (max - min).abs();
        if span <= f32::EPSILON {
            for row in 0..data.shape()[0] {
                data[(row, axis)] = 0.0;
            }
            continue;
        }

        for row in 0..data.shape()[0] {
            let normalized = (data[(row, axis)] - min) / span;
            data[(row, axis)] = normalized * 20.0 - 10.0;
        }
    }
}

fn normalize_points(points: &mut [[f32; 2]]) {
    if points.is_empty() {
        return;
    }

    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);

    for point in points.iter() {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let scale = ((max_x - min_x).abs().max((max_y - min_y).abs())) * 0.5;

    if scale <= f32::EPSILON {
        for point in points.iter_mut() {
            point[0] = 0.0;
            point[1] = 0.0;
        }
        return;
    }

    for point in points.iter_mut() {
        point[0] = (point[0] - center_x) / scale;
        point[1] = (point[1] - center_y) / scale;
    }
}

fn euclidean_distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| {
            let diff = a - b;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_vectors_to_2d_handles_small_inputs() {
        assert!(project_vectors_to_2d(&[]).unwrap().is_empty());
        assert_eq!(
            project_vectors_to_2d(&[vec![1.0, 2.0]]).unwrap(),
            vec![[0.0, 0.0]]
        );
        assert_eq!(
            project_vectors_to_2d(&[vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap(),
            vec![[-1.0, 0.0], [1.0, 0.0]]
        );
    }

    #[test]
    fn test_project_vectors_to_2d_returns_finite_normalized_points() {
        let points = project_vectors_to_2d(&[
            vec![1.0, 0.0, 0.0],
            vec![0.9, 0.1, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.9, 0.1],
        ])
        .unwrap();

        assert_eq!(points.len(), 4);
        assert!(points.iter().all(|[x, y]| {
            x.is_finite() && y.is_finite() && *x >= -1.0 && *x <= 1.0 && *y >= -1.0 && *y <= 1.0
        }));
    }
}
