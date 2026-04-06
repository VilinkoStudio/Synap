use crate::nlp::internal::{stable_map, stable_set, GramId, SparseVector, StableMap, StableSet};
use crate::nlp::tag::text::NgramTextEncoder;
use crate::nlp::traits::{LearnableTextEncoder, TextEncoder};
use crate::nlp::types::{NlpDocument, TagSuggestion};

const NGRAM_SIZE: usize = 3;
const GRAPH_WEIGHT: f32 = 0.1;
const SEED_LIMIT: usize = 5;
const SCORE_EPSILON: f32 = 1e-6;
const SCORE_PRECISION: f32 = 1_000_000.0;

#[derive(Debug, Clone)]
struct StoredDocument {
    tags: Vec<String>,
    vector: SparseVector,
}

#[derive(Debug)]
pub struct NlpTagIndex {
    documents: StableMap<String, StoredDocument>,
    text_encoder: NgramTextEncoder,
    tag_vectors: StableMap<String, SparseVector>,
    tag_name_vectors: StableMap<String, SparseVector>,
    gram_to_tags: StableMap<GramId, StableSet<String>>,
    tag_name_gram_to_tags: StableMap<GramId, StableSet<String>>,
    tag_doc_counts: StableMap<String, u32>,
    cooccur_graph: StableMap<String, StableMap<String, u32>>,
    doc_freq: StableMap<GramId, u32>,
    total_docs: u32,
}

impl Default for NlpTagIndex {
    fn default() -> Self {
        Self {
            documents: stable_map(),
            text_encoder: NgramTextEncoder::new(NGRAM_SIZE),
            tag_vectors: stable_map(),
            tag_name_vectors: stable_map(),
            gram_to_tags: stable_map(),
            tag_name_gram_to_tags: stable_map(),
            tag_doc_counts: stable_map(),
            cooccur_graph: stable_map(),
            doc_freq: stable_map(),
            total_docs: 0,
        }
    }
}

impl NlpTagIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn build<I>(&mut self, docs: I)
    where
        I: IntoIterator<Item = NlpDocument>,
    {
        self.clear();

        for doc in docs {
            self.upsert(doc);
        }
    }

    pub fn upsert(&mut self, doc: NlpDocument) {
        self.remove(&doc.id);

        let Some((id, stored)) = self.prepare_document(doc) else {
            return;
        };

        self.apply_insert(id, stored);
    }

    pub fn remove(&mut self, note_id: &str) -> bool {
        let Some(stored) = self.documents.remove(note_id) else {
            return false;
        };

        self.apply_remove(stored);
        true
    }

    pub fn suggest_tags(&self, content: &str, limit: usize) -> Vec<TagSuggestion> {
        if limit == 0 {
            return Vec::new();
        }

        let query_vector = self.text_encoder.encode(content);
        if query_vector.is_empty() {
            return Vec::new();
        }

        let query_weights = self.weight_query(&query_vector);
        if query_weights.is_empty() {
            return Vec::new();
        }

        let query_norm = vector_norm(&query_weights);
        if query_norm <= SCORE_EPSILON {
            return Vec::new();
        }

        let mut base_scores = stable_map();
        let mut lexical_scores = stable_map();
        for tag in self.collect_candidate_tags(query_weights.iter().map(|(gram, _)| *gram)) {
            let base_score = self
                .tag_vectors
                .get(&tag)
                .and_then(|vector| self.score_against_vector(vector, &query_weights, query_norm))
                .unwrap_or(0.0);
            let lexical_score = self
                .tag_name_vectors
                .get(&tag)
                .and_then(|vector| self.score_against_vector(vector, &query_weights, query_norm))
                .unwrap_or(0.0);

            if base_score > SCORE_EPSILON {
                base_scores.insert(tag.clone(), base_score);
            }
            if lexical_score > SCORE_EPSILON {
                lexical_scores.insert(tag, lexical_score);
            }
        }

        if base_scores.is_empty() && lexical_scores.is_empty() {
            return Vec::new();
        }

        let signal_scores = merge_signal_scores(&base_scores, &lexical_scores);
        let seeds = top_scored(&signal_scores, SEED_LIMIT);
        let mut candidate_tags: StableSet<String> = signal_scores.keys().cloned().collect();
        for (seed, _) in &seeds {
            if let Some(neighbors) = self.cooccur_graph.get(seed) {
                candidate_tags.extend(neighbors.keys().cloned());
            }
        }

        let mut suggestions = Vec::new();
        for tag in candidate_tags {
            let signal_score = *signal_scores.get(&tag).unwrap_or(&0.0);
            let graph_boost = self.graph_boost(&tag, &seeds);
            let final_score =
                quantize_score((signal_score + (GRAPH_WEIGHT * graph_boost)).clamp(0.0, 1.0));

            if final_score > SCORE_EPSILON {
                suggestions.push(TagSuggestion {
                    tag,
                    score: final_score,
                });
            }
        }

        suggestions.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.tag.cmp(&right.tag))
        });
        suggestions.truncate(limit);
        suggestions
    }

    pub fn recommend_tag(&self, content: &str, limit: usize) -> Vec<String> {
        self.suggest_tags(content, limit)
            .into_iter()
            .map(|suggestion| suggestion.tag)
            .collect()
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn tag_count(&self) -> usize {
        self.tag_vectors.len()
    }

    fn prepare_document(&mut self, doc: NlpDocument) -> Option<(String, StoredDocument)> {
        if !doc.active {
            return None;
        }

        let tags = normalize_tags(doc.tags);
        if tags.is_empty() {
            return None;
        }

        let vector = self.text_encoder.encode_with_updates(&doc.content);
        if vector.is_empty() {
            return None;
        }

        Some((doc.id, StoredDocument { tags, vector }))
    }

    fn apply_insert(&mut self, id: String, stored: StoredDocument) {
        self.total_docs += 1;

        for (gram, _) in &stored.vector {
            *self.doc_freq.entry(*gram).or_insert(0) += 1;
        }

        for tag in &stored.tags {
            let prior_count = self.tag_doc_counts.get(tag).copied().unwrap_or(0);
            *self.tag_doc_counts.entry(tag.clone()).or_insert(0) += 1;
            if prior_count == 0 {
                self.register_tag_name(tag);
            }

            let added_grams = {
                let tag_vector = self.tag_vectors.entry(tag.clone()).or_default();
                add_sparse_vector(tag_vector, &stored.vector)
            };

            for gram in added_grams {
                self.gram_to_tags
                    .entry(gram)
                    .or_default()
                    .insert(tag.clone());
            }
        }

        increment_cooccurrence(&mut self.cooccur_graph, &stored.tags);
        self.documents.insert(id, stored);
    }

    fn apply_remove(&mut self, stored: StoredDocument) {
        self.total_docs = self.total_docs.saturating_sub(1);

        for (gram, _) in &stored.vector {
            decrement_count(&mut self.doc_freq, gram);
        }

        for tag in &stored.tags {
            let removed_grams = match self.tag_vectors.get_mut(tag) {
                Some(tag_vector) => subtract_sparse_vector(tag_vector, &stored.vector),
                None => Vec::new(),
            };

            for gram in removed_grams {
                let should_remove = if let Some(tags) = self.gram_to_tags.get_mut(&gram) {
                    tags.remove(tag);
                    tags.is_empty()
                } else {
                    false
                };

                if should_remove {
                    self.gram_to_tags.remove(&gram);
                }
            }

            let should_remove_vector = self
                .tag_vectors
                .get(tag)
                .is_some_and(|vector| vector.is_empty());
            if should_remove_vector {
                self.tag_vectors.remove(tag);
            }

            decrement_count(&mut self.tag_doc_counts, tag);
            if !self.tag_doc_counts.contains_key(tag) {
                self.unregister_tag_name(tag);
            }
        }

        decrement_cooccurrence(&mut self.cooccur_graph, &stored.tags);
    }

    fn weight_query(&self, query_vector: &SparseVector) -> SparseVector {
        query_vector
            .iter()
            .filter_map(|(gram, tf)| {
                let weight = tf * self.idf(*gram);
                (weight > SCORE_EPSILON).then_some((*gram, weight))
            })
            .collect()
    }

    fn collect_candidate_tags(
        &self,
        query_grams: impl Iterator<Item = GramId>,
    ) -> StableSet<String> {
        let mut candidates = stable_set();

        for gram in query_grams {
            if let Some(tags) = self.gram_to_tags.get(&gram) {
                candidates.extend(tags.iter().cloned());
            }
            if let Some(tags) = self.tag_name_gram_to_tags.get(&gram) {
                candidates.extend(tags.iter().cloned());
            }
        }

        candidates
    }

    fn score_against_vector(
        &self,
        vector: &SparseVector,
        query_weights: &SparseVector,
        query_norm: f32,
    ) -> Option<f32> {
        let mut numerator = 0.0;
        let mut vector_norm_sq = 0.0;
        let mut query_index = 0usize;

        for (gram, tf) in vector {
            let weighted = tf * self.idf(*gram);
            vector_norm_sq += weighted * weighted;

            while query_index < query_weights.len() && query_weights[query_index].0 < *gram {
                query_index += 1;
            }

            if query_index < query_weights.len() && query_weights[query_index].0 == *gram {
                numerator += query_weights[query_index].1 * weighted;
            }
        }

        if numerator <= SCORE_EPSILON || vector_norm_sq <= SCORE_EPSILON {
            return None;
        }

        Some(quantize_score(
            (numerator / (query_norm * vector_norm_sq.sqrt())).clamp(0.0, 1.0),
        ))
    }

    fn graph_boost(&self, tag: &str, seeds: &[(String, f32)]) -> f32 {
        let mut boost = 0.0;

        for (seed, seed_score) in seeds {
            if seed == tag {
                continue;
            }

            let Some(count) = self
                .cooccur_graph
                .get(seed)
                .and_then(|neighbors| neighbors.get(tag))
            else {
                continue;
            };

            boost += seed_score * self.normalized_cooccur(seed, tag, *count);
        }

        quantize_score(boost.clamp(0.0, 1.0))
    }

    fn normalized_cooccur(&self, left: &str, right: &str, count: u32) -> f32 {
        let Some(left_count) = self.tag_doc_counts.get(left) else {
            return 0.0;
        };
        let Some(right_count) = self.tag_doc_counts.get(right) else {
            return 0.0;
        };

        let denom = ((*left_count as f32) * (*right_count as f32)).sqrt();
        if denom <= SCORE_EPSILON {
            return 0.0;
        }

        (count as f32) / denom
    }

    fn idf(&self, gram: GramId) -> f32 {
        let doc_count = self.total_docs.max(1) as f32;
        let doc_freq = self.doc_freq.get(&gram).copied().unwrap_or(0) as f32;
        ((1.0 + doc_count) / (1.0 + doc_freq)).ln() + 1.0
    }

    fn register_tag_name(&mut self, tag: &str) {
        let vector = self.text_encoder.encode_with_updates(tag);
        if vector.is_empty() {
            return;
        }

        for (gram, _) in &vector {
            self.tag_name_gram_to_tags
                .entry(*gram)
                .or_default()
                .insert(tag.to_owned());
        }

        self.tag_name_vectors.insert(tag.to_owned(), vector);
    }

    fn unregister_tag_name(&mut self, tag: &str) {
        let Some(vector) = self.tag_name_vectors.remove(tag) else {
            return;
        };

        for (gram, _) in &vector {
            let should_remove = if let Some(tags) = self.tag_name_gram_to_tags.get_mut(gram) {
                tags.remove(tag);
                tags.is_empty()
            } else {
                false
            };

            if should_remove {
                self.tag_name_gram_to_tags.remove(gram);
            }
        }
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = stable_set();
    let mut normalized = Vec::with_capacity(tags.len());

    for raw in tags {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if seen.insert(trimmed.to_owned()) {
            normalized.push(trimmed.to_owned());
        }
    }

    normalized
}

fn vector_norm(vector: &SparseVector) -> f32 {
    quantize_score(
        vector
            .iter()
            .map(|(_, value)| value * value)
            .sum::<f32>()
            .sqrt(),
    )
}

fn top_scored(scores: &StableMap<String, f32>, limit: usize) -> Vec<(String, f32)> {
    let mut items: Vec<(String, f32)> = scores
        .iter()
        .map(|(tag, score)| (tag.clone(), *score))
        .collect();
    items.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    items.truncate(limit);
    items
}

fn merge_signal_scores(
    base_scores: &StableMap<String, f32>,
    lexical_scores: &StableMap<String, f32>,
) -> StableMap<String, f32> {
    let mut merged = stable_map();

    for (tag, score) in base_scores {
        merged.insert(tag.clone(), *score);
    }

    for (tag, lexical_score) in lexical_scores {
        let entry = merged.entry(tag.clone()).or_insert(0.0);
        *entry = entry.max(*lexical_score);
    }

    merged
}

fn decrement_count<K>(counts: &mut StableMap<K, u32>, key: &K)
where
    K: Eq + std::hash::Hash + Clone,
{
    let should_remove = if let Some(value) = counts.get_mut(key) {
        *value = value.saturating_sub(1);
        *value == 0
    } else {
        false
    };

    if should_remove {
        counts.remove(key);
    }
}

fn increment_cooccurrence(graph: &mut StableMap<String, StableMap<String, u32>>, tags: &[String]) {
    for i in 0..tags.len() {
        for j in (i + 1)..tags.len() {
            let left = &tags[i];
            let right = &tags[j];

            *graph
                .entry(left.clone())
                .or_default()
                .entry(right.clone())
                .or_insert(0) += 1;
            *graph
                .entry(right.clone())
                .or_default()
                .entry(left.clone())
                .or_insert(0) += 1;
        }
    }
}

fn decrement_cooccurrence(graph: &mut StableMap<String, StableMap<String, u32>>, tags: &[String]) {
    for i in 0..tags.len() {
        for j in (i + 1)..tags.len() {
            let left = &tags[i];
            let right = &tags[j];
            decrement_nested_count(graph, left, right);
            decrement_nested_count(graph, right, left);
        }
    }
}

fn decrement_nested_count(
    graph: &mut StableMap<String, StableMap<String, u32>>,
    outer: &str,
    inner: &str,
) {
    let should_remove_outer = if let Some(neighbors) = graph.get_mut(outer) {
        decrement_count(neighbors, &inner.to_string());
        neighbors.is_empty()
    } else {
        false
    };

    if should_remove_outer {
        graph.remove(outer);
    }
}

fn add_sparse_vector(target: &mut SparseVector, delta: &SparseVector) -> Vec<GramId> {
    let mut merged = Vec::with_capacity(target.len() + delta.len());
    let mut added = Vec::new();
    let mut left = 0usize;
    let mut right = 0usize;

    while left < target.len() && right < delta.len() {
        match target[left].0.cmp(&delta[right].0) {
            std::cmp::Ordering::Less => {
                merged.push(target[left]);
                left += 1;
            }
            std::cmp::Ordering::Greater => {
                merged.push(delta[right]);
                added.push(delta[right].0);
                right += 1;
            }
            std::cmp::Ordering::Equal => {
                merged.push((target[left].0, target[left].1 + delta[right].1));
                left += 1;
                right += 1;
            }
        }
    }

    while left < target.len() {
        merged.push(target[left]);
        left += 1;
    }

    while right < delta.len() {
        merged.push(delta[right]);
        added.push(delta[right].0);
        right += 1;
    }

    *target = merged;
    added
}

fn subtract_sparse_vector(target: &mut SparseVector, delta: &SparseVector) -> Vec<GramId> {
    let mut merged = Vec::with_capacity(target.len());
    let mut removed = Vec::new();
    let mut left = 0usize;
    let mut right = 0usize;

    while left < target.len() {
        if right >= delta.len() || target[left].0 < delta[right].0 {
            merged.push(target[left]);
            left += 1;
            continue;
        }

        if target[left].0 > delta[right].0 {
            right += 1;
            continue;
        }

        let updated = target[left].1 - delta[right].1;
        if updated > SCORE_EPSILON {
            merged.push((target[left].0, updated));
        } else {
            removed.push(target[left].0);
        }

        left += 1;
        right += 1;
    }

    *target = merged;
    removed
}

fn quantize_score(score: f32) -> f32 {
    (score * SCORE_PRECISION).round() / SCORE_PRECISION
}
