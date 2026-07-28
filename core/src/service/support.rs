use super::*;
use crate::models::{
    embedding_cache::EmbeddingCacheStamp,
    tag_profile::{TagProfileDocumentRecord, TagProfileMetadata, TagProfileTag},
};

impl SynapService {
    /// 封装只读事务的生命周期
    pub(crate) fn with_read<F, T>(&self, f: F) -> Result<T, ServiceError>
    where
        // 闭包接收事务和 Reader，返回你的目标类型 T
        F: FnOnce(&ReadTransaction, &NoteReader<'_>) -> Result<T, ServiceError>,
    {
        let tx = self.db.begin_read()?;
        let reader = NoteReader::new(&tx)?;
        f(&tx, &reader) // 执行你的核心业务逻辑
    }

    /// 封装写入事务的生命周期
    pub(crate) fn with_write<F, T>(&self, f: F) -> Result<T, ServiceError>
    where
        F: FnOnce(&WriteTransaction) -> Result<T, ServiceError>,
    {
        let tx = self.db.begin_write()?;
        let result = f(&tx)?;
        tx.commit()?; // 自动提交！
        Ok(result)
    }

    // UUID 解析辅助函数，告别满屏的 Uuid::parse_str
    pub(crate) fn parse_id(id: &str) -> Result<Uuid, ServiceError> {
        Uuid::parse_str(id).map_err(Into::into)
    }

    pub(crate) fn parse_ids(ids: &[String]) -> Result<Vec<Uuid>, ServiceError> {
        ids.iter().map(|id| Self::parse_id(id)).collect()
    }

    pub(crate) fn normalize_tag_inputs(tags: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
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

    pub(crate) fn materialize_tags(
        &self,
        tx: &WriteTransaction,
        tags: Vec<String>,
    ) -> Result<Vec<Tag>, ServiceError> {
        let tag_writer = TagWriter::new(tx);

        Self::normalize_tag_inputs(tags)
            .into_iter()
            .map(|tag| tag_writer.find_or_create(tag).map_err(Into::into))
            .collect()
    }

    pub(crate) fn rebuild_tag_search(&self) -> Result<(), ServiceError> {
        self.tag_searcher.clear();
        self.with_read(|tx, _reader| {
            let tag_reader = TagReader::new(tx)?;
            let tags = tag_reader
                .all()
                .map_err(redb::Error::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;
            self.tag_searcher.insert_batch(tags.into_iter());
            Ok(())
        })
    }

    pub(crate) fn ensure_starmap_model_ready(&self) -> Result<(), ServiceError> {
        let snapshot = self.with_read(|tx, reader| {
            let view = StarmapView::new(tx, reader);
            if view.needs_initial_model()? {
                return Ok(Some(view.build_full_snapshot()?));
            }
            Ok(None)
        })?;

        let Some(snapshot) = snapshot else {
            return Ok(());
        };

        self.with_write(|wtx| StarmapView::persist_snapshot(wtx, &snapshot))
    }

    pub(crate) fn remove_starmap_note(&self, note_id: Uuid) -> Result<(), ServiceError> {
        let model = self.with_read(|tx, reader| {
            let view = StarmapView::new(tx, reader);
            view.remove_note_from_model(note_id)
        })?;

        self.with_write(|wtx| {
            UmapCache::delete_point(wtx, &note_id.into_bytes())?;
            if let Some(model) = model {
                StarmapView::persist_model(wtx, &model)?;
            } else {
                UmapCache::clear_model(wtx)?;
            }
            Ok(())
        })
    }

    pub(crate) fn rebuild_starmap_full_cache(&self) -> Result<(), ServiceError> {
        let snapshot =
            self.with_read(|tx, reader| StarmapView::new(tx, reader).build_full_snapshot())?;
        self.with_write(|wtx| StarmapView::persist_snapshot(wtx, &snapshot))
    }

    fn expected_embedding_stamp(&self) -> EmbeddingCacheStamp {
        let config = self.config.lock().expect("config lock");
        EmbeddingCacheStamp::new(
            config.embedding.space_fingerprint(),
            config.embedding.dimension(),
        )
    }

    fn expected_tag_profile_metadata(&self) -> TagProfileMetadata {
        let stamp = self.expected_embedding_stamp();
        TagProfileMetadata::new(stamp.space_fingerprint, stamp.dimension as usize)
    }

    /// Adopts compatible legacy vectors and invalidates derived state when the
    /// persisted embedding space or cache format no longer matches.
    pub(crate) fn initialize_derived_caches(&self) -> Result<(), ServiceError> {
        let expected_embedding = self.expected_embedding_stamp();
        let expected_profiles = self.expected_tag_profile_metadata();
        let (embedding_stamp, legacy_vectors_compatible, profile_metadata) =
            self.with_read(|tx, _reader| {
                let stamp = EmbeddingCacheMetadata::load(tx)?;
                let mut compatible = true;
                if stamp.is_none() {
                    for item in Note::vector_index().iter(tx)? {
                        let (_, vector) = item.map_err(redb::Error::from)?;
                        if !vector.is_empty()
                            && (vector.len() != expected_embedding.dimension as usize
                                || vector.iter().any(|value| !value.is_finite()))
                        {
                            compatible = false;
                            break;
                        }
                    }
                }
                Ok((stamp, compatible, TagProfileStore::load_metadata(tx)?))
            })?;

        let embedding_incompatible = embedding_stamp
            .as_ref()
            .is_some_and(|stamp| !stamp.is_compatible_with(&expected_embedding))
            || (embedding_stamp.is_none() && !legacy_vectors_compatible);
        let profiles_incompatible = profile_metadata.as_ref().is_none_or(|metadata| {
            !metadata.is_compatible_with(&expected_profiles) || !metadata.is_ready()
        });

        self.with_write(|tx| {
            if embedding_incompatible {
                self.semantic_index.invalidate_all(tx)?;
                UmapCache::clear_points(tx)?;
                UmapCache::clear_model(tx)?;
            }
            if embedding_stamp.is_none() || embedding_incompatible {
                EmbeddingCacheMetadata::save(tx, &expected_embedding)?;
            }
            if profiles_incompatible || embedding_incompatible {
                TagProfileStore::reset(tx, &expected_profiles)?;
            }
            Ok(())
        })?;

        self.reconcile_note_embeddings()?;
        if profiles_incompatible || embedding_incompatible {
            self.rebuild_tag_profiles_from_vectors()?;
        }
        self.reload_tag_profile_index()
    }

    /// Migration/recovery path only: joins live latest notes with already
    /// persisted NoteVectors and publishes a complete profile snapshot.
    pub(crate) fn rebuild_tag_profiles_from_vectors(&self) -> Result<(), ServiceError> {
        let expected = self.expected_tag_profile_metadata();
        let documents = self.with_read(|tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut documents = Vec::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if note_ref.is_deleted() || !Self::is_latest_version(reader, note_ref)? {
                    continue;
                }
                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                let Some(vector) = self
                    .semantic_index
                    .get(tx, &note.get_id().into_bytes())?
                    .filter(|vector| !SemanticIndex::is_pending(vector))
                else {
                    continue;
                };
                if vector.len() != expected.dimension as usize {
                    continue;
                }
                let tags = NoteView::new(reader, note.clone())
                    .tags()?
                    .into_iter()
                    .map(|tag| TagProfileTag {
                        id: tag.get_id().into_bytes(),
                        name: tag.get_content().to_owned(),
                    })
                    .collect::<Vec<_>>();
                if tags.is_empty() {
                    continue;
                }
                documents.push((
                    note.get_id().into_bytes(),
                    TagProfileDocumentRecord {
                        tags,
                        embedding: vector,
                    },
                ));
            }
            Ok(documents)
        })?;

        self.with_write(|tx| {
            TagProfileStore::replace_all(tx, &expected, documents)?;
            Ok(())
        })
    }

    pub(crate) fn reload_tag_profile_index(&self) -> Result<(), ServiceError> {
        let _lifecycle = match self.embedding_lifecycle.try_read() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
            Err(std::sync::TryLockError::Poisoned(error)) => {
                panic!("embedding lifecycle lock: {error}")
            }
        };
        self.reload_tag_profile_index_locked()
    }

    /// Caller must hold either side of embedding_lifecycle, which prevents an
    /// old embedding-space snapshot from being published across config changes.
    pub(crate) fn reload_tag_profile_index_locked(&self) -> Result<(), ServiceError> {
        let snapshot = self.with_read(|tx, _reader| {
            let metadata = TagProfileStore::load_metadata(tx)?;
            let profiles = TagProfileStore::load_profiles(tx)?;
            let preference = TagProfileStore::load_preference_model(tx)?;
            Ok(metadata.map(|metadata| (metadata, profiles, preference)))
        })?;
        let index = snapshot
            .map(|(metadata, profiles, preference)| {
                TagProfileIndex::build(&metadata, profiles, preference)
            })
            .unwrap_or_default();
        self.tag_recommender.replace_if_newer(index);
        Ok(())
    }

    pub(crate) fn refresh_tag_indexes(&self) -> Result<(), ServiceError> {
        self.rebuild_tag_search()?;
        self.reload_tag_profile_index()
    }

    //传None代表临时文件
    pub fn new(db_path: Option<String>) -> Result<Self, ServiceError> {
        let db = db_path.map_or_else(
            || -> Result<Database, ServiceError> {
                let file = NamedTempFile::new().map_err(|_| ServiceError::TempfileIO(()))?;
                Ok(Database::create(file.path()).map_err(|err| ServiceError::Db(err.into()))?)
            },
            |path| -> Result<Database, ServiceError> {
                let p = Path::new(&path);
                if p.exists() {
                    Ok(Database::open(p).map_err(|err| ServiceError::Db(err.into()))?)
                } else {
                    Database::create(p).map_err(|err| ServiceError::Db(err.into()))
                }
            },
        )?;

        let tx = db
            .begin_write()
            .map_err(|err| ServiceError::Db(err.into()))?;
        Note::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        UmapCache::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        TagWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        CryptoWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        RelayPeerWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        SyncStatsWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        ConfigWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        EmbeddingCacheMetadata::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        TagProfileStore::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        crypto::ensure_local_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        crypto::ensure_local_signing_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        let core_config = ConfigWriter::new(&tx).load_or_default()?;
        tx.commit().map_err(ServiceError::CommitErr)?;

        let embedding_model = Self::build_embedding_model(&core_config.embedding)?;
        let tag_searcher = FuzzyIndex::<Tag>::new();
        let note_searcher = FuzzyIndex::<Note>::new();
        let semantic_index = SemanticIndex::new(Note::vector_index(), embedding_model);
        let tag_recommender = ServiceTagRecommender::new();

        let res = Self {
            db,
            embedding_lifecycle: RwLock::new(()),
            config: Mutex::new(core_config),
            tag_searcher,
            note_searcher,
            semantic_index,
            tag_recommender,
        };
        res.initialize_derived_caches()?;
        res.init_search()?;
        Ok(res)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ServiceError> {
        Self::new(Some(path.as_ref().to_string_lossy().into_owned()))
    }

    pub fn open_memory() -> Result<Self, ServiceError> {
        Self::new(None)
    }
}
