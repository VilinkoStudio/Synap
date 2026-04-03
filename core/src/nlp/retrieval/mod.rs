use std::sync::Arc;

use crate::db::types::BlockId;
use crate::db::vector::VectorStore;
use crate::nlp::embedding::EmbeddingModel;

pub struct VectorRetrieval {
    vector_store: Arc<VectorStore<Vec<f32>>>,
    embedding_model: Arc<dyn EmbeddingModel>,
}

impl VectorRetrieval {
    pub fn new(
        vector_store: VectorStore<Vec<f32>>,
        embedding_model: Arc<dyn EmbeddingModel>,
    ) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            embedding_model,
        }
    }

    pub fn search(
        &self,
        tx: &redb::ReadTransaction,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, redb::Error> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query_vector = self.embedding_model.encode(query);
        if query_vector.is_empty() {
            return Ok(Vec::new());
        }

        let query_norm = vector_norm(&query_vector);
        if query_norm == 0.0 {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let iter = self.vector_store.iter(tx)?;

        for item in iter {
            let (key_guard, doc_vector) = item.map_err(|e| redb::Error::from(e))?;
            let similarity = cosine_similarity(&query_vector, &doc_vector, query_norm);
            if similarity > 0.0 {
                let note_id = key_guard.value();
                let note_id_array: BlockId = note_id.try_into().unwrap();
                results.push(SearchResult {
                    note_id: note_id_array,
                    score: similarity,
                });
            }
        }

        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(limit);
        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: BlockId,
    pub score: f32,
}

fn vector_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine_similarity(a: &[f32], b: &[f32], a_norm: f32) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let b_norm = vector_norm(b);
    if b_norm == 0.0 {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot / (a_norm * b_norm)
}
