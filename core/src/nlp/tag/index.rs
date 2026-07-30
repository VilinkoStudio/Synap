use crate::{
    models::tag_profile::{
        validate_metric, TagMetricRecord, TagPreferenceModelRecord, TagProfileMetadata,
        TagProfileRecord, TAG_PREFERENCE_MODEL_VERSION,
    },
    nlp::{metrics::vector_norm, types::TagSuggestion},
};

const SCORE_EPSILON: f32 = 1e-6;

#[derive(Debug, Default)]
pub(crate) struct TagProfileIndex {
    dimension: usize,
    embedding_space: String,
    generation: u64,
    metric: RuntimeMetric,
    profiles: Vec<IndexedTagProfile>,
}

#[derive(Debug)]
struct IndexedTagProfile {
    tag: String,
    projected: Vec<f32>,
}

#[derive(Debug, Default)]
enum RuntimeMetric {
    #[default]
    Identity,
    SemanticSpectral {
        dimension: usize,
        rank: usize,
        basis: Vec<f32>,
        half_scale_offsets: Vec<f32>,
    },
    HashDiagonal {
        half_weights: Vec<f32>,
    },
}

impl TagProfileIndex {
    pub fn build(
        metadata: &TagProfileMetadata,
        records: Vec<TagProfileRecord>,
        preference: Option<TagPreferenceModelRecord>,
    ) -> Self {
        let dimension = metadata.dimension as usize;
        let metric = RuntimeMetric::from_record(
            dimension,
            metadata.generation,
            &metadata.embedding_space,
            preference,
        );
        let profiles = records
            .into_iter()
            .filter(|record| record.embedding_count > 0 && record.embedding_sum.len() == dimension)
            .filter_map(|record| {
                let projected = metric.project_normalized(&record.embedding_sum)?;
                Some(IndexedTagProfile {
                    tag: record.name,
                    projected,
                })
            })
            .collect();

        Self {
            dimension,
            embedding_space: metadata.embedding_space.clone(),
            generation: metadata.generation,
            metric,
            profiles,
        }
    }

    pub fn suggest_tags(&self, query: &[f32], limit: usize) -> Vec<TagSuggestion> {
        if limit == 0 || query.len() != self.dimension {
            return Vec::new();
        }
        let Some(query) = self.metric.project_normalized(query) else {
            return Vec::new();
        };

        let mut suggestions = self
            .profiles
            .iter()
            .filter_map(|profile| {
                let score = query
                    .iter()
                    .zip(&profile.projected)
                    .map(|(left, right)| left * right)
                    .sum::<f32>();
                (score > SCORE_EPSILON).then(|| TagSuggestion {
                    tag: profile.tag.clone(),
                    score,
                })
            })
            .collect::<Vec<_>>();
        suggestions.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.tag.cmp(&right.tag))
        });
        suggestions.truncate(limit);
        suggestions
    }

    pub fn recommend_tags(&self, query: &[f32], limit: usize) -> Vec<String> {
        self.suggest_tags(query, limit)
            .into_iter()
            .map(|suggestion| suggestion.tag)
            .collect()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn embedding_space(&self) -> &str {
        &self.embedding_space
    }

    pub fn tag_count(&self) -> usize {
        self.profiles.len()
    }
}

impl RuntimeMetric {
    fn from_record(
        dimension: usize,
        generation: u64,
        embedding_space: &str,
        preference: Option<TagPreferenceModelRecord>,
    ) -> Self {
        let Some(preference) = preference else {
            return Self::Identity;
        };
        if preference.version != TAG_PREFERENCE_MODEL_VERSION
            || preference.embedding_space != embedding_space
            || preference.trained_on_generation != generation
            || validate_metric(&preference.metric, dimension).is_err()
        {
            return Self::Identity;
        }

        match preference.metric {
            TagMetricRecord::Identity => Self::Identity,
            TagMetricRecord::SemanticSpectral {
                dimension: _,
                rank,
                basis,
                log_scales,
            } => {
                let rank = rank as usize;
                Self::SemanticSpectral {
                    dimension,
                    rank,
                    basis,
                    half_scale_offsets: log_scales
                        .into_iter()
                        .map(|value| (0.5 * value).exp() - 1.0)
                        .collect(),
                }
            }
            TagMetricRecord::HashDiagonal { log_weights } => Self::HashDiagonal {
                half_weights: log_weights
                    .into_iter()
                    .map(|value| (0.5 * value).exp())
                    .collect(),
            },
        }
    }

    fn project_normalized(&self, input: &[f32]) -> Option<Vec<f32>> {
        if input.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let mut output = match self {
            Self::Identity => input.to_vec(),
            Self::HashDiagonal { half_weights } => input
                .iter()
                .zip(half_weights)
                .map(|(value, weight)| value * weight)
                .collect(),
            Self::SemanticSpectral {
                dimension,
                rank,
                basis,
                half_scale_offsets,
            } => {
                if input.len() != *dimension {
                    return None;
                }
                let mut output = input.to_vec();
                for column in 0..*rank {
                    let column_start = column * dimension;
                    let basis_column = &basis[column_start..column_start + dimension];
                    let coefficient = basis_column
                        .iter()
                        .zip(input)
                        .map(|(basis, value)| basis * value)
                        .sum::<f32>()
                        * half_scale_offsets[column];
                    for (value, basis) in output.iter_mut().zip(basis_column) {
                        *value += coefficient * basis;
                    }
                }
                output
            }
        };
        let norm = vector_norm(&output);
        if norm <= SCORE_EPSILON || !norm.is_finite() {
            return None;
        }
        for value in &mut output {
            *value /= norm;
        }
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::tag_profile::{
        TagMetricRecord, TagPreferenceModelRecord, MAX_ABS_LOG_METRIC_SCALE,
    };

    use super::*;

    fn metadata() -> TagProfileMetadata {
        TagProfileMetadata {
            schema_version: 1,
            algorithm_version: 1,
            embedding_space: "semantic:test".into(),
            dimension: 3,
            generation: 7,
        }
    }

    fn profile(id: u8, name: &str, vector: [f32; 3]) -> TagProfileRecord {
        TagProfileRecord {
            tag_id: [id; 16],
            name: name.into(),
            embedding_sum: vector.to_vec(),
            embedding_count: 1,
        }
    }

    #[test]
    fn identity_metric_ranks_profile_cosine() {
        let index = TagProfileIndex::build(
            &metadata(),
            vec![
                profile(1, "rust", [1.0, 0.0, 0.0]),
                profile(2, "database", [0.0, 1.0, 0.0]),
            ],
            None,
        );

        assert_eq!(index.recommend_tags(&[0.9, 0.1, 0.0], 2)[0], "rust");
        assert_eq!(index.generation(), 7);
    }

    #[test]
    fn semantic_metric_uses_persisted_spectral_projection() {
        let preference = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "semantic:test".into(),
            trained_on_generation: 7,
            metric: TagMetricRecord::SemanticSpectral {
                dimension: 3,
                rank: 1,
                basis: vec![1.0, 0.0, 0.0],
                log_scales: vec![4.0_f32.ln()],
            },
        };
        let index = TagProfileIndex::build(
            &metadata(),
            vec![
                profile(1, "x", [0.6, 0.8, 0.0]),
                profile(2, "y", [0.2, 0.98, 0.0]),
            ],
            Some(preference),
        );

        assert_eq!(index.recommend_tags(&[1.0, 0.0, 0.0], 2)[0], "x");
    }

    #[test]
    fn invalid_preference_falls_back_to_identity() {
        let preference = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "other-space".into(),
            trained_on_generation: 1,
            metric: TagMetricRecord::HashDiagonal {
                log_weights: vec![1.0; 3],
            },
        };
        let index = TagProfileIndex::build(
            &metadata(),
            vec![profile(1, "rust", [1.0, 0.0, 0.0])],
            Some(preference),
        );
        assert_eq!(index.recommend_tags(&[1.0, 0.0, 0.0], 1), ["rust"]);
    }

    #[test]
    fn stale_preference_generation_falls_back_to_identity() {
        let preference = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "semantic:test".into(),
            trained_on_generation: 6,
            metric: TagMetricRecord::SemanticSpectral {
                dimension: 3,
                rank: 1,
                basis: vec![1.0, 0.0, 0.0],
                log_scales: vec![0.5],
            },
        };
        let index = TagProfileIndex::build(
            &metadata(),
            vec![profile(1, "rust", [1.0, 0.0, 0.0])],
            Some(preference),
        );
        assert!(matches!(index.metric, RuntimeMetric::Identity));
    }

    #[test]
    fn out_of_range_metric_falls_back_to_identity() {
        let preference = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "semantic:test".into(),
            trained_on_generation: 7,
            metric: TagMetricRecord::SemanticSpectral {
                dimension: 3,
                rank: 1,
                basis: vec![1.0, 0.0, 0.0],
                log_scales: vec![MAX_ABS_LOG_METRIC_SCALE + 0.01],
            },
        };
        let index = TagProfileIndex::build(
            &metadata(),
            vec![profile(1, "rust", [1.0, 0.0, 0.0])],
            Some(preference),
        );
        assert!(matches!(index.metric, RuntimeMetric::Identity));
        assert_eq!(index.recommend_tags(&[1.0, 0.0, 0.0], 1), ["rust"]);
    }

    #[test]
    fn non_orthonormal_metric_falls_back_to_identity() {
        let preference = TagPreferenceModelRecord {
            version: TAG_PREFERENCE_MODEL_VERSION,
            embedding_space: "semantic:test".into(),
            trained_on_generation: 7,
            metric: TagMetricRecord::SemanticSpectral {
                dimension: 3,
                rank: 1,
                basis: vec![2.0, 0.0, 0.0],
                log_scales: vec![0.5],
            },
        };
        let index = TagProfileIndex::build(
            &metadata(),
            vec![profile(1, "rust", [1.0, 0.0, 0.0])],
            Some(preference),
        );
        assert!(matches!(index.metric, RuntimeMetric::Identity));
    }
}
