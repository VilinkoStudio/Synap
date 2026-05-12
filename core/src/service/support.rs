use super::*;

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

    pub(crate) fn rebuild_note_embeddings(&self) -> Result<(), ServiceError> {
        let notes = self.with_read(|_tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, note_ref)? || note_ref.is_deleted() {
                    continue;
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                notes.push((
                    note.get_id().into_bytes(),
                    Cow::Owned(note.get_search_text()),
                ));
            }

            Ok(notes)
        })?;

        self.with_write(|tx| {
            self.semantic_index.rebuild(tx, notes)?;
            Ok(())
        })
    }

    pub(crate) fn index_note_embedding(&self, note: &Note) -> Result<(), ServiceError> {
        let note_id = note.get_id().into_bytes();
        let content = note.get_search_text();
        self.with_write(|tx| {
            self.semantic_index.upsert(tx, &note_id, &content)?;
            Ok(())
        })
    }

    pub(crate) fn note_embedding(&self, note_id: Uuid) -> Result<Option<Vec<f32>>, ServiceError> {
        self.with_read(|tx, _reader| {
            Note::vector_index()
                .get(tx, &note_id.into_bytes())
                .map_err(Into::into)
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

    pub(crate) fn upsert_starmap_note(&self, note_id: Uuid) -> Result<(), ServiceError> {
        let Some(vector) = self.note_embedding(note_id)? else {
            return Ok(());
        };

        let snapshot = self.with_read(|tx, reader| {
            let view = StarmapView::new(tx, reader);
            view.upsert_note_from_model(note_id, vector)
        })?;

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

    pub(crate) fn note_to_nlp_document(
        note: Note,
        reader: &NoteReader<'_>,
    ) -> Result<Option<NlpDocument>, ServiceError> {
        if note.is_deleted() {
            return Ok(None);
        }

        let id = note.get_id().to_string();
        let content = note.content().to_string();
        let tags = NoteView::new(reader, note)
            .tags()?
            .into_iter()
            .map(|tag| tag.get_content().to_string())
            .collect::<Vec<_>>();

        if tags.is_empty() {
            return Ok(None);
        }

        Ok(Some(NlpDocument::new(id, content, tags)))
    }

    pub(crate) fn collect_tag_recommendation_docs(&self) -> Result<Vec<NlpDocument>, ServiceError> {
        self.with_read(|_tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut docs = Vec::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, note_ref)? {
                    continue;
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                if let Some(doc) = Self::note_to_nlp_document(note, reader)? {
                    docs.push(doc);
                }
            }

            Ok(docs)
        })
    }

    pub(crate) fn rebuild_tag_recommender(&self) -> Result<(), ServiceError> {
        let docs = self.collect_tag_recommendation_docs()?;
        self.tag_recommender.rebuild(docs);
        Ok(())
    }

    pub(crate) fn refresh_tag_indexes(&self) -> Result<(), ServiceError> {
        self.rebuild_tag_search()?;
        self.rebuild_tag_recommender()
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
        crypto::ensure_local_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        crypto::ensure_local_signing_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        tx.commit().map_err(ServiceError::CommitErr)?;

        let tag_searcher = FuzzyIndex::<Tag>::new();
        let note_searcher = FuzzyIndex::<Note>::new();
        let semantic_index =
            SemanticIndex::new(Note::vector_index(), Arc::new(LocalHashEmbedding::new(0)));
        let tag_recommender = ServiceTagRecommender::new();

        let res = Self {
            db,
            tag_searcher,
            note_searcher,
            semantic_index,
            tag_recommender,
        };
        res.refresh_search_indexes()?;
        Ok(res)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ServiceError> {
        Self::new(Some(path.as_ref().to_string_lossy().into_owned()))
    }

    pub fn open_memory() -> Result<Self, ServiceError> {
        Self::new(None)
    }
}
