use std::sync::{Arc, RwLock};

use crate::db::types::BlockId;
use crate::db::vector::VectorStore;
use crate::error::ServiceError;
use crate::nlp::embedding::EmbeddingModel;
use crate::nlp::metrics::{cosine_similarity, vector_norm};

pub struct SemanticIndex {
    vector_store: Arc<VectorStore<Vec<f32>>>,
    embedding_model: RwLock<Arc<dyn EmbeddingModel>>,
}

impl SemanticIndex {
    pub fn new(
        vector_store: VectorStore<Vec<f32>>,
        embedding_model: Arc<dyn EmbeddingModel>,
    ) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            embedding_model: RwLock::new(embedding_model),
        }
    }

    pub fn set_embedding_model(&self, embedding_model: Arc<dyn EmbeddingModel>) {
        *self.embedding_model.write().expect("embedding model lock") = embedding_model;
    }

    fn embedding_model(&self) -> Arc<dyn EmbeddingModel> {
        self.embedding_model
            .read()
            .expect("embedding model lock")
            .clone()
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, ServiceError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        self.embedding_model().embed(text).map_err(Into::into)
    }

    /// 语义搜索：暴力扫描全部向量 → 按余弦相似度排序
    pub fn search_vector(
        &self,
        tx: &redb::ReadTransaction,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        if limit == 0 || query_vector.is_empty() {
            return Ok(Vec::new());
        }

        let query_norm = vector_norm(query_vector);
        if query_norm == 0.0 {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let iter = self.vector_store.iter(tx).map_err(ServiceError::from)?;

        for item in iter {
            let (key_guard, doc_vector) = item.map_err(redb::Error::from)?;
            let similarity = cosine_similarity(query_vector, &doc_vector, query_norm);
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

    /// 预留空向量占位，不计算 embedding（由 backfill 异步补全）
    pub fn reserve(
        &self,
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
    ) -> Result<(), ServiceError> {
        self.vector_store
            .put(tx, note_id, &Vec::new())
            .map_err(ServiceError::from)
    }

    pub fn is_pending(vector: &[f32]) -> bool {
        vector.is_empty()
    }

    pub fn get(
        &self,
        tx: &redb::ReadTransaction,
        note_id: &BlockId,
    ) -> Result<Option<Vec<f32>>, ServiceError> {
        self.vector_store
            .get(tx, note_id)
            .map_err(ServiceError::from)
    }

    pub fn put_embedding(
        &self,
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
        vector: &[f32],
    ) -> Result<bool, ServiceError> {
        if vector.is_empty() {
            self.vector_store
                .delete(tx, note_id)
                .map_err(ServiceError::from)?;
            return Ok(false);
        }

        self.vector_store
            .put(tx, note_id, &vector.to_vec())
            .map_err(ServiceError::from)?;
        Ok(true)
    }

    pub fn delete(
        &self,
        tx: &redb::WriteTransaction,
        note_id: &BlockId,
    ) -> Result<bool, ServiceError> {
        self.vector_store
            .delete(tx, note_id)
            .map_err(ServiceError::from)
    }

    /// 将已有向量槽全部置空（pending），保留 key。
    pub fn invalidate_all(&self, tx: &redb::WriteTransaction) -> Result<usize, ServiceError> {
        use redb::ReadableTable;

        let keys = {
            let table = tx
                .open_table(self.vector_store.table_def())
                .map_err(redb::Error::from)?;
            table
                .range::<BlockId>(..)
                .map_err(redb::Error::from)?
                .map(|res| res.map(|(key_guard, _)| key_guard.value()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?
        };

        for key in &keys {
            self.vector_store.put(tx, key, &Vec::new())?;
        }
        Ok(keys.len())
    }

    /// TODO: 联想 —— 找与指定 note 最相似的其他笔记
    pub fn find_similar(
        &self,
        _tx: &redb::ReadTransaction,
        _note_id: BlockId,
        _limit: usize,
    ) -> Result<Vec<SearchResult>, ServiceError> {
        todo!("find_similar")
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: BlockId,
    pub score: f32,
}
