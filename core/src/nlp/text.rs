use crate::text::sanitize_search_text;

use super::types::{gram_weights, GramId, SparseVector};

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

fn counts_to_sparse_vector(mut counts: super::types::GramWeights) -> SparseVector {
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
