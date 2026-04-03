use crate::text::sanitize_search_text;

use crate::nlp::internal::{gram_weights, GramId, SparseVector, StableMap};
use crate::nlp::traits::{LearnableTextEncoder, TextEncoder};

#[derive(Debug)]
pub(super) struct NgramTextEncoder {
    ngram_size: usize,
    gram_ids: StableMap<String, GramId>,
    next_gram_id: GramId,
}

impl NgramTextEncoder {
    pub(super) fn new(ngram_size: usize) -> Self {
        Self {
            ngram_size,
            gram_ids: StableMap::default(),
            next_gram_id: 0,
        }
    }

    fn encode_with_resolver<F>(&self, text: &str, resolve: F) -> SparseVector
    where
        F: FnMut(&str) -> Option<GramId>,
    {
        build_term_frequencies_with_resolver(text, self.ngram_size, resolve)
    }

    fn resolve_or_insert_gram(&mut self, gram: &str) -> GramId {
        if let Some(id) = self.gram_ids.get(gram).copied() {
            return id;
        }

        let id = self.next_gram_id;
        self.next_gram_id = self.next_gram_id.saturating_add(1);
        self.gram_ids.insert(gram.to_owned(), id);
        id
    }

    fn lookup_gram(&self, gram: &str) -> Option<GramId> {
        self.gram_ids.get(gram).copied()
    }
}

impl TextEncoder<SparseVector> for NgramTextEncoder {
    fn encode(&self, text: &str) -> SparseVector {
        self.encode_with_resolver(text, |gram| self.lookup_gram(gram))
    }
}

impl LearnableTextEncoder<SparseVector> for NgramTextEncoder {
    fn encode_with_updates(&mut self, text: &str) -> SparseVector {
        let normalized = normalize_nlp_text(text);
        build_term_frequencies_from_normalized_with_resolver(&normalized, self.ngram_size, |gram| {
            Some(self.resolve_or_insert_gram(gram))
        })
    }
}

pub(super) fn normalize_nlp_text(content: &str) -> String {
    sanitize_search_text(content).to_lowercase()
}

pub(super) fn build_term_frequencies_with_resolver<F>(
    content: &str,
    ngram_size: usize,
    resolve: F,
) -> SparseVector
where
    F: FnMut(&str) -> Option<GramId>,
{
    let normalized = normalize_nlp_text(content);
    build_term_frequencies_from_normalized_with_resolver(&normalized, ngram_size, resolve)
}

pub(super) fn build_term_frequencies_from_normalized_with_resolver<F>(
    normalized: &str,
    ngram_size: usize,
    mut resolve: F,
) -> SparseVector
where
    F: FnMut(&str) -> Option<GramId>,
{
    if normalized.is_empty() || ngram_size == 0 {
        return Vec::new();
    }

    let mut counts = gram_weights();
    for_each_char_ngram(normalized, ngram_size, |gram| {
        if let Some(id) = resolve_gram(&mut resolve, gram) {
            *counts.entry(id).or_insert(0.0) += 1.0;
        }
    });

    counts_to_sparse_vector(counts)
}

fn resolve_gram<F>(resolver: &mut F, gram: &str) -> Option<GramId>
where
    F: FnMut(&str) -> Option<GramId>,
{
    resolver(gram)
}

fn counts_to_sparse_vector(mut counts: crate::nlp::internal::GramWeights) -> SparseVector {
    let mut vector: SparseVector = counts
        .drain()
        .map(|(gram_id, tf)| (gram_id, 1.0 + tf.ln()))
        .collect();
    vector.sort_unstable_by_key(|(gram_id, _)| *gram_id);
    vector
}

fn for_each_char_ngram<F>(text: &str, ngram_size: usize, mut callback: F)
where
    F: FnMut(&str),
{
    if ngram_size == 0 {
        return;
    }

    let mut boundaries = Vec::with_capacity(text.len().min(64) + 1);
    boundaries.extend(text.char_indices().map(|(idx, _)| idx));
    if boundaries.is_empty() {
        return;
    }

    boundaries.push(text.len());
    let char_count = boundaries.len() - 1;

    if char_count <= ngram_size {
        callback(text);
        return;
    }

    for start_idx in 0..=(char_count - ngram_size) {
        let start = boundaries[start_idx];
        let end = boundaries[start_idx + ngram_size];
        callback(&text[start..end]);
    }
}
