use super::*;
use crate::models::{
    config::{ConfigWriter, EmbeddingConfig},
    embedding_cache::{EmbeddingCacheMetadata, EmbeddingCacheStamp},
    tag_profile::{TagProfileDocumentRecord, TagProfileMetadata, TagProfileStore, TagProfileTag},
};
use crate::nlp::embedding::{EmbeddingModel, LocalHashEmbedding, OpenAiEmbeddingModel};
use std::time::Duration;

/// backfill 进度回调：每处理完一条 pending 槽位调用一次。
pub type EmbeddingBackfillProgressCallback<'a> =
    dyn FnMut(EmbeddingBackfillProgressDTO) + Send + 'a;

impl SynapService {
    pub(crate) fn build_embedding_model(
        config: &EmbeddingConfig,
    ) -> Result<Arc<dyn EmbeddingModel>, ServiceError> {
        match config {
            EmbeddingConfig::LocalHash { dimension } => {
                Ok(Arc::new(LocalHashEmbedding::new(*dimension)))
            }
            EmbeddingConfig::Http {
                endpoint,
                api_key,
                model,
                dimension,
                timeout_ms,
            } => Ok(Arc::new(
                OpenAiEmbeddingModel::new(api_key.clone(), model.clone(), *dimension)
                    .with_endpoint(endpoint.clone())
                    .with_timeout(Duration::from_millis(*timeout_ms)),
            )),
        }
    }

    /// 当前 embedding 配置（强类型 → DTO）。
    pub fn get_embedding_config(&self) -> EmbeddingConfigDTO {
        let _lifecycle = self
            .embedding_lifecycle
            .read()
            .expect("embedding lifecycle lock");
        let config = self.config.lock().expect("config lock").clone();
        embedding_config_to_dto(&config.embedding)
    }

    /// 更新 embedding 配置：落盘 → 注入模型 → 全部向量置空。
    ///
    /// 调用方随后应跑 [`backfill_note_embeddings_with_progress`] 重填。
    pub fn set_embedding_config(
        &self,
        config: EmbeddingConfigDTO,
    ) -> Result<EmbeddingConfigDTO, ServiceError> {
        let embedding = embedding_config_from_dto(config)?;
        let model = Self::build_embedding_model(&embedding)?;
        let _lifecycle = self
            .embedding_lifecycle
            .write()
            .expect("embedding lifecycle lock");
        let next = {
            let guard = self.config.lock().expect("config lock");
            let mut updated = guard.clone();
            updated.embedding = embedding;
            updated.validate()?;
            updated
        };
        let embedding_stamp = EmbeddingCacheStamp::new(
            next.embedding.space_fingerprint(),
            next.embedding.dimension(),
        );
        let profile_metadata = TagProfileMetadata::new(
            embedding_stamp.space_fingerprint.clone(),
            embedding_stamp.dimension as usize,
        );

        let update_result = self.with_write(|tx| {
            ConfigWriter::new(tx).save(&next)?;
            self.semantic_index.invalidate_all(tx)?;
            EmbeddingCacheMetadata::save(tx, &embedding_stamp)?;
            TagProfileStore::reset(tx, &profile_metadata)?;
            Ok(())
        });
        if let Err(error) = update_result {
            // A concurrent profile reload may have yielded to this config writer.
            // Republish the unchanged persisted snapshot before returning failure.
            self.reload_tag_profile_index_locked()?;
            return Err(error);
        }

        self.semantic_index.set_embedding_model(model);
        self.tag_recommender.clear();
        *self.config.lock().expect("config lock") = next.clone();

        Ok(embedding_config_to_dto(&next.embedding))
    }

    /// 为笔记写入空向量占位，真正的 embedding 由 backfill 补全。
    pub(crate) fn reserve_note_embedding(&self, note: &Note) -> Result<(), ServiceError> {
        let note_id = note.get_id().into_bytes();
        if note.get_search_text().trim().is_empty() {
            self.with_write(|tx| {
                self.semantic_index.delete(tx, &note_id)?;
                TagProfileStore::remove_document(tx, &note_id)?;
                Ok(())
            })?;
            return Ok(());
        }

        self.with_write(|tx| {
            self.semantic_index.reserve(tx, &note_id)?;
            TagProfileStore::remove_document(tx, &note_id)?;
            Ok(())
        })
    }

    pub(crate) fn delete_note_embedding(&self, note_id: Uuid) -> Result<(), ServiceError> {
        self.with_write(|tx| {
            self.semantic_index.delete(tx, &note_id.into_bytes())?;
            TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
            Ok(())
        })?;
        self.reload_tag_profile_index()
    }

    /// 对齐向量表与 live 笔记：
    /// - live latest 缺失槽位 → 预留空向量
    /// - 非 live / 已删除 / 旧版本槽位 → 删除
    /// - 已有非空向量保留不动
    pub(crate) fn reconcile_note_embeddings(&self) -> Result<(), ServiceError> {
        let (live_ids, existing_ids) = self.with_read(|tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut live_ids = HashSet::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, note_ref)? || note_ref.is_deleted() {
                    continue;
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                if note.get_search_text().trim().is_empty() {
                    continue;
                }
                live_ids.insert(note.get_id());
            }

            let mut existing_ids = HashSet::new();
            for item in Note::vector_index().iter(tx)? {
                let (key_guard, _vector) = item.map_err(redb::Error::from)?;
                existing_ids.insert(Uuid::from_bytes(key_guard.value()));
            }

            Ok((live_ids, existing_ids))
        })?;

        self.with_write(|tx| {
            for note_id in &existing_ids {
                if !live_ids.contains(note_id) {
                    self.semantic_index.delete(tx, &note_id.into_bytes())?;
                    TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
                }
            }

            for note_id in &live_ids {
                if !existing_ids.contains(note_id) {
                    self.semantic_index.reserve(tx, &note_id.into_bytes())?;
                    TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
                }
            }

            Ok(())
        })?;
        self.reload_tag_profile_index()
    }

    /// 扫描空向量占位并插补 embedding（无进度回调）。
    pub fn backfill_note_embeddings(&self) -> Result<usize, ServiceError> {
        self.backfill_note_embeddings_with_progress(&mut |_| {})
    }

    /// 扫描空向量占位并插补 embedding，每条处理后回调进度。
    pub fn backfill_note_embeddings_with_progress(
        &self,
        on_progress: &mut EmbeddingBackfillProgressCallback<'_>,
    ) -> Result<usize, ServiceError> {
        let _lifecycle = self
            .embedding_lifecycle
            .read()
            .expect("embedding lifecycle lock");
        let pending = self.with_read(|tx, reader| {
            let mut pending = Vec::new();

            for item in Note::vector_index().iter(tx)? {
                let (key_guard, vector) = item.map_err(redb::Error::from)?;
                if !SemanticIndex::is_pending(&vector) {
                    continue;
                }

                let note_id = Uuid::from_bytes(key_guard.value());
                let Some(note_ref) = reader.get_ref_by_id(&note_id)? else {
                    pending.push((note_id, None));
                    continue;
                };

                if note_ref.is_deleted() || !Self::is_latest_version(reader, note_ref)? {
                    pending.push((note_id, None));
                    continue;
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_id.to_string()))?;
                let text = note.get_search_text();
                if text.trim().is_empty() {
                    pending.push((note_id, None));
                } else {
                    pending.push((note_id, Some(text)));
                }
            }

            Ok(pending)
        })?;

        let total = pending.len();
        let mut processed = 0usize;
        let mut filled_count = 0usize;
        let mut skipped = 0usize;

        // Embedding runs outside redb transactions. The short write transaction
        // revalidates the immutable note version before publishing the vector.
        for (note_id, text) in pending {
            let current_note_id = Some(note_id.to_string());

            let filled_result = match text.as_deref() {
                Some(text) => self
                    .semantic_index
                    .embed(text)
                    .and_then(|vector| self.commit_note_embedding(note_id, text, &vector)),
                None => self.reconcile_pending_note_embedding(note_id),
            };
            let filled = match filled_result {
                Ok(filled) => filled,
                Err(error) => {
                    self.reload_tag_profile_index_locked()?;
                    return Err(error);
                }
            };

            processed += 1;
            if filled {
                filled_count += 1;
            } else {
                skipped += 1;
            }

            on_progress(EmbeddingBackfillProgressDTO {
                total,
                processed,
                filled: filled_count,
                skipped,
                current_note_id,
            });
        }

        if filled_count > 0 || skipped > 0 {
            self.reload_tag_profile_index_locked()?;
        }

        Ok(filled_count)
    }

    pub(crate) fn commit_note_embedding(
        &self,
        note_id: Uuid,
        expected_text: &str,
        vector: &[f32],
    ) -> Result<bool, ServiceError> {
        self.with_write(
            |tx| match Note::embedding_source_if_live_latest_in_write(tx, &note_id)? {
                Some(source) if source.text == expected_text => {
                    let stored =
                        self.semantic_index
                            .put_embedding(tx, &note_id.into_bytes(), vector)?;
                    if !stored {
                        TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
                        return Ok(false);
                    }

                    let tag_writer = TagWriter::new(tx);
                    let mut tags = Vec::with_capacity(source.tag_ids.len());
                    for tag_id in source.tag_ids {
                        let tag = tag_writer.get_by_id(&tag_id)?.ok_or_else(|| {
                            redb::Error::Io(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "note references a missing tag",
                            ))
                        })?;
                        tags.push(TagProfileTag {
                            id: tag_id.into_bytes(),
                            name: tag.get_content().to_owned(),
                        });
                    }
                    TagProfileStore::upsert_document(
                        tx,
                        &note_id.into_bytes(),
                        TagProfileDocumentRecord {
                            tags,
                            embedding: vector.to_vec(),
                        },
                    )?;
                    Ok(true)
                }
                Some(_) => {
                    self.semantic_index.reserve(tx, &note_id.into_bytes())?;
                    TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
                    Ok(false)
                }
                None => {
                    self.semantic_index.delete(tx, &note_id.into_bytes())?;
                    TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
                    Ok(false)
                }
            },
        )
    }

    fn reconcile_pending_note_embedding(&self, note_id: Uuid) -> Result<bool, ServiceError> {
        self.with_write(|tx| {
            if Note::embedding_text_if_live_latest_in_write(tx, &note_id)?.is_some() {
                self.semantic_index.reserve(tx, &note_id.into_bytes())?;
            } else {
                self.semantic_index.delete(tx, &note_id.into_bytes())?;
            }
            TagProfileStore::remove_document(tx, &note_id.into_bytes())?;
            Ok(false)
        })
    }
}

pub(crate) fn embedding_config_to_dto(config: &EmbeddingConfig) -> EmbeddingConfigDTO {
    match config {
        EmbeddingConfig::LocalHash { dimension } => EmbeddingConfigDTO {
            provider: EmbeddingProviderDTO::LocalHash,
            dimension: *dimension,
            endpoint: None,
            api_key: None,
            model: None,
            timeout_ms: None,
        },
        EmbeddingConfig::Http {
            endpoint,
            api_key,
            model,
            dimension,
            timeout_ms,
        } => EmbeddingConfigDTO {
            provider: EmbeddingProviderDTO::Http,
            dimension: *dimension,
            endpoint: Some(endpoint.clone()),
            api_key: Some(api_key.clone()),
            model: Some(model.clone()),
            timeout_ms: Some(*timeout_ms),
        },
    }
}

pub(crate) fn embedding_config_from_dto(
    dto: EmbeddingConfigDTO,
) -> Result<EmbeddingConfig, ServiceError> {
    let config = match dto.provider {
        EmbeddingProviderDTO::LocalHash => EmbeddingConfig::LocalHash {
            dimension: dto.dimension,
        },
        EmbeddingProviderDTO::Http => EmbeddingConfig::Http {
            endpoint: dto.endpoint.unwrap_or_default(),
            api_key: dto.api_key.unwrap_or_default(),
            model: dto.model.unwrap_or_default(),
            dimension: dto.dimension,
            timeout_ms: dto.timeout_ms.unwrap_or(30_000),
        },
    };
    config.validate()?;
    Ok(config)
}
