use std::collections::HashSet;

use redb::{ReadTransaction, ReadableTable, WriteTransaction};
use serde::{Deserialize, Serialize};

use crate::db::{
    kvstore::{KvReader, KvStore},
    types::BlockId,
};

const TAG_PROFILE_METADATA: KvStore<u8, TagProfileMetadata> = KvStore::new("TagProfileMetadata");
const TAG_PROFILE_STORE: KvStore<BlockId, TagProfileRecord> = KvStore::new("TagProfiles");
const TAG_PROFILE_DOCUMENT_STORE: KvStore<BlockId, TagProfileDocumentRecord> =
    KvStore::new("TagProfileDocuments");
const TAG_PREFERENCE_MODEL: KvStore<u8, TagPreferenceModelRecord> =
    KvStore::new("TagPreferenceModel");
const SINGLETON_KEY: u8 = 0;

pub(crate) const TAG_PROFILE_SCHEMA_VERSION: u32 = 1;
pub(crate) const TAG_PROFILE_ALGORITHM_VERSION: u32 = 1;
pub(crate) const TAG_PREFERENCE_MODEL_VERSION: u32 = 1;
pub(crate) const MAX_ABS_LOG_METRIC_SCALE: f32 = 1.386_294_4; // ln(4)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TagProfileMetadata {
    pub schema_version: u32,
    pub algorithm_version: u32,
    pub embedding_space: String,
    pub dimension: u32,
    pub generation: u64,
}

impl TagProfileMetadata {
    pub fn new(embedding_space: String, dimension: usize) -> Self {
        Self {
            schema_version: TAG_PROFILE_SCHEMA_VERSION,
            algorithm_version: TAG_PROFILE_ALGORITHM_VERSION,
            embedding_space,
            dimension: dimension as u32,
            generation: 0,
        }
    }

    pub fn is_compatible_with(&self, expected: &Self) -> bool {
        self.schema_version == expected.schema_version
            && self.algorithm_version == expected.algorithm_version
            && self.embedding_space == expected.embedding_space
            && self.dimension == expected.dimension
    }

    /// Generation zero is the durable "building" marker. A cache snapshot is
    /// published only after replace_all commits a positive generation.
    pub fn is_ready(&self) -> bool {
        self.generation > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TagProfileRecord {
    pub tag_id: BlockId,
    pub name: String,
    pub embedding_sum: Vec<f32>,
    pub embedding_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) struct TagProfileTag {
    pub id: BlockId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TagProfileDocumentRecord {
    pub tags: Vec<TagProfileTag>,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TagPreferenceModelRecord {
    pub version: u32,
    pub embedding_space: String,
    pub trained_on_generation: u64,
    pub metric: TagMetricRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum TagMetricRecord {
    Identity,
    SemanticSpectral {
        dimension: u32,
        rank: u32,
        /// Column-major d x r orthonormal basis.
        basis: Vec<f32>,
        log_scales: Vec<f32>,
    },
    HashDiagonal {
        log_weights: Vec<f32>,
    },
}

pub(crate) struct TagProfileStore;

impl TagProfileStore {
    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        TAG_PROFILE_METADATA.init_table(tx)?;
        TAG_PROFILE_STORE.init_table(tx)?;
        TAG_PROFILE_DOCUMENT_STORE.init_table(tx)?;
        TAG_PREFERENCE_MODEL.init_table(tx)?;
        Ok(())
    }

    pub fn load_metadata(tx: &ReadTransaction) -> Result<Option<TagProfileMetadata>, redb::Error> {
        let reader: KvReader<u8, TagProfileMetadata> = TAG_PROFILE_METADATA.reader(tx)?;
        reader.get(&SINGLETON_KEY)
    }

    pub fn load_profiles(tx: &ReadTransaction) -> Result<Vec<TagProfileRecord>, redb::Error> {
        let reader = TAG_PROFILE_STORE.reader(tx)?;
        reader
            .iter()?
            .map(|item| item.map(|(_, profile)| profile).map_err(redb::Error::from))
            .collect()
    }

    pub fn load_preference_model(
        tx: &ReadTransaction,
    ) -> Result<Option<TagPreferenceModelRecord>, redb::Error> {
        let reader: KvReader<u8, TagPreferenceModelRecord> = TAG_PREFERENCE_MODEL.reader(tx)?;
        reader.get(&SINGLETON_KEY)
    }

    pub fn save_preference_model(
        tx: &WriteTransaction,
        model: &TagPreferenceModelRecord,
    ) -> Result<(), redb::Error> {
        let metadata = metadata_in_write(tx)?;
        if !metadata.is_ready() {
            return Err(invalid_data(
                "cannot publish tag preference for a building profile snapshot",
            ));
        }
        if model.version != TAG_PREFERENCE_MODEL_VERSION {
            return Err(invalid_data("unsupported tag preference model version"));
        }
        if model.embedding_space != metadata.embedding_space {
            return Err(invalid_data("tag preference embedding space mismatch"));
        }
        if model.trained_on_generation != metadata.generation {
            return Err(invalid_data(
                "tag preference was trained on a stale profile snapshot",
            ));
        }
        match &model.metric {
            TagMetricRecord::SemanticSpectral { .. }
                if !metadata.embedding_space.starts_with("http:") =>
            {
                return Err(invalid_data(
                    "semantic tag metric requires a semantic embedding space",
                ));
            }
            TagMetricRecord::HashDiagonal { .. }
                if !metadata.embedding_space.starts_with("local-hash:") =>
            {
                return Err(invalid_data(
                    "hash tag metric requires a local hash embedding space",
                ));
            }
            _ => {}
        }
        validate_metric(&model.metric, metadata.dimension as usize)?;
        TAG_PREFERENCE_MODEL.put(tx, &SINGLETON_KEY, model)
    }

    pub fn reset(tx: &WriteTransaction, metadata: &TagProfileMetadata) -> Result<(), redb::Error> {
        clear_block_store(tx, TAG_PROFILE_STORE)?;
        clear_block_store(tx, TAG_PROFILE_DOCUMENT_STORE)?;
        clear_u8_store(tx, TAG_PREFERENCE_MODEL)?;
        TAG_PROFILE_METADATA.put(tx, &SINGLETON_KEY, metadata)
    }

    pub fn replace_all(
        tx: &WriteTransaction,
        metadata: &TagProfileMetadata,
        documents: impl IntoIterator<Item = (BlockId, TagProfileDocumentRecord)>,
    ) -> Result<(), redb::Error> {
        Self::reset(tx, metadata)?;
        for (note_id, document) in documents {
            upsert_document_inner(tx, &note_id, document, metadata.dimension as usize)?;
        }
        let mut persisted = metadata.clone();
        persisted.generation = 1;
        TAG_PROFILE_METADATA.put(tx, &SINGLETON_KEY, &persisted)
    }

    pub fn upsert_document(
        tx: &WriteTransaction,
        note_id: &BlockId,
        document: TagProfileDocumentRecord,
    ) -> Result<(), redb::Error> {
        let mut metadata = metadata_in_write(tx)?;
        upsert_document_inner(tx, note_id, document, metadata.dimension as usize)?;
        metadata.generation = metadata.generation.saturating_add(1);
        TAG_PREFERENCE_MODEL.delete(tx, &SINGLETON_KEY)?;
        TAG_PROFILE_METADATA.put(tx, &SINGLETON_KEY, &metadata)
    }

    pub fn remove_document(tx: &WriteTransaction, note_id: &BlockId) -> Result<bool, redb::Error> {
        let removed = remove_document_inner(tx, note_id)?;
        if removed {
            let mut metadata = metadata_in_write(tx)?;
            metadata.generation = metadata.generation.saturating_add(1);
            TAG_PREFERENCE_MODEL.delete(tx, &SINGLETON_KEY)?;
            TAG_PROFILE_METADATA.put(tx, &SINGLETON_KEY, &metadata)?;
        }
        Ok(removed)
    }
}

fn metadata_in_write(tx: &WriteTransaction) -> Result<TagProfileMetadata, redb::Error> {
    TAG_PROFILE_METADATA
        .get_in_write(tx, &SINGLETON_KEY)?
        .ok_or_else(|| invalid_data("tag profile metadata is missing"))
}

fn upsert_document_inner(
    tx: &WriteTransaction,
    note_id: &BlockId,
    mut document: TagProfileDocumentRecord,
    dimension: usize,
) -> Result<(), redb::Error> {
    remove_document_inner(tx, note_id)?;

    normalize_in_place(&mut document.embedding, dimension)?;
    if document.embedding.iter().all(|value| *value == 0.0) {
        return Ok(());
    }

    let mut seen = HashSet::new();
    document.tags.retain(|tag| seen.insert(tag.id));
    if document.tags.is_empty() {
        return Ok(());
    }

    for tag in &document.tags {
        let mut profile = TAG_PROFILE_STORE
            .get_in_write(tx, &tag.id)?
            .unwrap_or_else(|| TagProfileRecord {
                tag_id: tag.id,
                name: tag.name.clone(),
                embedding_sum: vec![0.0; dimension],
                embedding_count: 0,
            });
        if profile.embedding_sum.len() != dimension {
            return Err(invalid_data("tag profile dimension mismatch"));
        }
        profile.name.clone_from(&tag.name);
        for (sum, value) in profile.embedding_sum.iter_mut().zip(&document.embedding) {
            *sum += *value;
        }
        profile.embedding_count = profile.embedding_count.saturating_add(1);
        TAG_PROFILE_STORE.put(tx, &tag.id, &profile)?;
    }

    TAG_PROFILE_DOCUMENT_STORE.put(tx, note_id, &document)
}

fn remove_document_inner(tx: &WriteTransaction, note_id: &BlockId) -> Result<bool, redb::Error> {
    let Some(document) = TAG_PROFILE_DOCUMENT_STORE.get_in_write(tx, note_id)? else {
        return Ok(false);
    };

    for tag in &document.tags {
        let Some(mut profile) = TAG_PROFILE_STORE.get_in_write(tx, &tag.id)? else {
            continue;
        };
        if profile.embedding_count <= 1 {
            TAG_PROFILE_STORE.delete(tx, &tag.id)?;
            continue;
        }
        if profile.embedding_sum.len() != document.embedding.len() {
            return Err(invalid_data("tag profile contribution dimension mismatch"));
        }
        for (sum, value) in profile.embedding_sum.iter_mut().zip(&document.embedding) {
            *sum -= *value;
        }
        profile.embedding_count -= 1;
        TAG_PROFILE_STORE.put(tx, &tag.id, &profile)?;
    }

    TAG_PROFILE_DOCUMENT_STORE.delete(tx, note_id)
}

fn normalize_in_place(vector: &mut [f32], dimension: usize) -> Result<(), redb::Error> {
    if vector.len() != dimension {
        return Err(invalid_data("tag profile input dimension mismatch"));
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(invalid_data("tag profile input contains non-finite values"));
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for value in vector {
            *value /= norm;
        }
    }
    Ok(())
}

pub(crate) fn validate_metric(
    metric: &TagMetricRecord,
    dimension: usize,
) -> Result<(), redb::Error> {
    match metric {
        TagMetricRecord::Identity => Ok(()),
        TagMetricRecord::HashDiagonal { log_weights } => {
            if log_weights.len() != dimension
                || log_weights
                    .iter()
                    .any(|weight| !weight.is_finite() || weight.abs() > MAX_ABS_LOG_METRIC_SCALE)
            {
                return Err(invalid_data("invalid hash diagonal tag metric"));
            }
            Ok(())
        }
        TagMetricRecord::SemanticSpectral {
            dimension: stored_dimension,
            rank,
            basis,
            log_scales,
        } => {
            let rank = *rank as usize;
            if *stored_dimension as usize != dimension
                || rank == 0
                || rank > dimension
                || basis.len() != dimension * rank
                || log_scales.len() != rank
                || basis.iter().any(|value| !value.is_finite())
                || log_scales
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() > MAX_ABS_LOG_METRIC_SCALE)
            {
                return Err(invalid_data("invalid semantic spectral tag metric"));
            }
            for left in 0..rank {
                for right in 0..rank {
                    let dot = (0..dimension)
                        .map(|row| basis[left * dimension + row] * basis[right * dimension + row])
                        .sum::<f32>();
                    let expected = if left == right { 1.0 } else { 0.0 };
                    if (dot - expected).abs() > 1e-3 {
                        return Err(invalid_data("semantic tag metric basis is not orthonormal"));
                    }
                }
            }
            Ok(())
        }
    }
}

fn clear_block_store<V>(
    tx: &WriteTransaction,
    store: KvStore<BlockId, V>,
) -> Result<(), redb::Error>
where
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut table = tx.open_table(store.table_def())?;
    let keys = table
        .range::<BlockId>(..)?
        .map(|item| item.map(|(key, _)| key.value()))
        .collect::<Result<Vec<_>, _>>()?;
    for key in keys {
        table.remove(key)?;
    }
    Ok(())
}

fn clear_u8_store<V>(tx: &WriteTransaction, store: KvStore<u8, V>) -> Result<(), redb::Error>
where
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut table = tx.open_table(store.table_def())?;
    let keys = table
        .range::<u8>(..)?
        .map(|item| item.map(|(key, _)| key.value()))
        .collect::<Result<Vec<_>, _>>()?;
    for key in keys {
        table.remove(key)?;
    }
    Ok(())
}

fn invalid_data(message: &'static str) -> redb::Error {
    redb::Error::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;

    fn setup() -> Database {
        let file = NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let tx = db.begin_write().unwrap();
        TagProfileStore::init_schema(&tx).unwrap();
        TagProfileStore::reset(
            &tx,
            &TagProfileMetadata::new("local-hash:test-space".into(), 2),
        )
        .unwrap();
        tx.commit().unwrap();
        db
    }

    fn document(tag_id: BlockId, name: &str, embedding: [f32; 2]) -> TagProfileDocumentRecord {
        TagProfileDocumentRecord {
            tags: vec![TagProfileTag {
                id: tag_id,
                name: name.into(),
            }],
            embedding: embedding.to_vec(),
        }
    }

    #[test]
    fn profile_contributions_are_incremental_and_reversible() {
        let db = setup();
        let tag_id = [7; 16];
        let first_id = [1; 16];
        let second_id = [2; 16];

        let tx = db.begin_write().unwrap();
        TagProfileStore::upsert_document(&tx, &first_id, document(tag_id, "rust", [3.0, 4.0]))
            .unwrap();
        TagProfileStore::upsert_document(&tx, &second_id, document(tag_id, "rust", [0.0, 2.0]))
            .unwrap();
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        let profiles = TagProfileStore::load_profiles(&tx).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].embedding_count, 2);
        assert_eq!(profiles[0].embedding_sum, vec![0.6, 1.8]);
        drop(tx);

        let tx = db.begin_write().unwrap();
        assert!(TagProfileStore::remove_document(&tx, &first_id).unwrap());
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        let profiles = TagProfileStore::load_profiles(&tx).unwrap();
        assert_eq!(profiles[0].embedding_count, 1);
        assert!(profiles[0].embedding_sum[0].abs() < 1e-6);
        assert!((profiles[0].embedding_sum[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn replacing_a_document_replaces_its_profile_contribution() {
        let db = setup();
        let note_id = [1; 16];
        let old_tag = [7; 16];
        let new_tag = [8; 16];

        let tx = db.begin_write().unwrap();
        TagProfileStore::upsert_document(&tx, &note_id, document(old_tag, "old", [1.0, 0.0]))
            .unwrap();
        TagProfileStore::upsert_document(&tx, &note_id, document(new_tag, "new", [0.0, 1.0]))
            .unwrap();
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        let profiles = TagProfileStore::load_profiles(&tx).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].tag_id, new_tag);
    }

    #[test]
    fn preference_publish_rejects_stale_profile_generation() {
        let db = setup();
        let tx = db.begin_write().unwrap();
        let model = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "local-hash:test-space".into(),
            trained_on_generation: 99,
            metric: TagMetricRecord::Identity,
        };
        assert!(TagProfileStore::save_preference_model(&tx, &model).is_err());
    }

    #[test]
    fn preference_publish_roundtrips_current_snapshot() {
        let db = setup();
        let tx = db.begin_write().unwrap();
        TagProfileStore::upsert_document(&tx, &[1; 16], document([7; 16], "rust", [1.0, 0.0]))
            .unwrap();
        let model = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "local-hash:test-space".into(),
            trained_on_generation: 1,
            metric: TagMetricRecord::HashDiagonal {
                log_weights: vec![0.25, -0.25],
            },
        };
        TagProfileStore::save_preference_model(&tx, &model).unwrap();
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        assert_eq!(
            TagProfileStore::load_preference_model(&tx).unwrap(),
            Some(model)
        );
    }

    #[test]
    fn preference_publish_rejects_out_of_range_metric_scale() {
        let db = setup();
        let tx = db.begin_write().unwrap();
        TagProfileStore::upsert_document(&tx, &[1; 16], document([7; 16], "rust", [1.0, 0.0]))
            .unwrap();
        let model = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "local-hash:test-space".into(),
            trained_on_generation: 1,
            metric: TagMetricRecord::HashDiagonal {
                log_weights: vec![MAX_ABS_LOG_METRIC_SCALE + 0.01, 0.0],
            },
        };
        assert!(TagProfileStore::save_preference_model(&tx, &model).is_err());
    }

    #[test]
    fn preference_publish_rejects_building_snapshot() {
        let db = setup();
        let tx = db.begin_write().unwrap();
        let model = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "local-hash:test-space".into(),
            trained_on_generation: 0,
            metric: TagMetricRecord::Identity,
        };
        assert!(TagProfileStore::save_preference_model(&tx, &model).is_err());
    }

    #[test]
    fn profile_mutation_invalidates_preference_model() {
        let db = setup();
        let tx = db.begin_write().unwrap();
        TagProfileStore::upsert_document(&tx, &[1; 16], document([7; 16], "rust", [1.0, 0.0]))
            .unwrap();
        let model = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "local-hash:test-space".into(),
            trained_on_generation: 1,
            metric: TagMetricRecord::Identity,
        };
        TagProfileStore::save_preference_model(&tx, &model).unwrap();
        TagProfileStore::upsert_document(&tx, &[2; 16], document([7; 16], "rust", [0.0, 1.0]))
            .unwrap();
        tx.commit().unwrap();

        let tx = db.begin_read().unwrap();
        assert!(TagProfileStore::load_preference_model(&tx)
            .unwrap()
            .is_none());
    }
}
