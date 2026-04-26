use std::{borrow::Cow, sync::Arc};

use crate::db::types::BlockId;
use crate::db::vector::VectorStore;
use crate::nlp::embedding::EmbeddingModel;
use crate::nlp::metrics::{cosine_similarity, vector_norm};

pub struct SemanticIndex {
    vector_store: Arc<VectorStore<Vec<f32>>>,
    embedding_model: Arc<dyn EmbeddingModel>,
}

impl SemanticIndex {
    pub fn new(
        vector_store: VectorStore<Vec<f32>>,
        embedding_model: Arc<dyn EmbeddingModel>,
    ) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            embedding_model,
        }
    }

    /// 语义搜索：对 query 做 embedding → 暴力扫描全部向量 → 按余弦相似度排序
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

    pub fn upsert(
        &self,
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
        text: &str,
    ) -> Result<bool, redb::Error> {
        let text = text.trim();
        if text.is_empty() {
            self.vector_store.delete(tx, note_id)?;
            return Ok(false);
        }

        let vector = self.embedding_model.embed(text);
        if vector.is_empty() {
            self.vector_store.delete(tx, note_id)?;
            return Ok(false);
        }

        self.vector_store.put(tx, note_id, &vector)?;
        Ok(true)
    }

    pub fn delete(
        &self,
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
    ) -> Result<bool, redb::Error> {
        self.vector_store.delete(tx, note_id)
    }

    pub fn rebuild<'a, I>(
        &self,
        tx: &redb::WriteTransaction,
        documents: I,
    ) -> Result<usize, redb::Error>
    where
        I: IntoIterator<Item = (BlockId, Cow<'a, str>)>,
    {
        self.vector_store.clear(tx)?;

        let mut indexed = 0;
        for (note_id, text) in documents {
            if self.upsert(tx, &note_id, text.as_ref())? {
                indexed += 1;
            }
        }

        Ok(indexed)
    }

    /// TODO: 联想 —— 找与指定 note 最相似的其他笔记
    pub fn find_similar(
        &self,
        _tx: &redb::ReadTransaction,
        _note_id: BlockId,
        _limit: usize,
    ) -> Result<Vec<SearchResult>, redb::Error> {
        todo!("find_similar")
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: BlockId,
    pub score: f32,
}
