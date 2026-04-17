use std::{
    collections::HashSet,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};

use crate::{
    crypto,
    dto::{
        LocalIdentityDTO, NoteDTO, PeerDTO, PeerTrustStatusDTO, PublicKeyInfoDTO, ShareStatsDTO,
        SyncSessionDTO, SyncStatsDTO, SyncStatusDTO, TimelineNotesPageDTO, TimelineSessionDTO,
        TimelineSessionsPageDTO,
    },
    error::ServiceError,
    models::{
        crypto::{CryptoReader, CryptoWriter},
        note::{Note, NoteReader, NoteRef},
        tag::{Tag, TagReader, TagWriter},
    },
    nlp::{NlpDocument, NlpTagIndex},
    search::searcher::FuzzyIndex,
    sync::{ShareService, SyncService},
    views::{
        note_view::NoteView,
        timeline_view::{SessionDetectionConfig, SessionSpan, TimelinePoint, TimelineView},
    },
};

use redb::{Database, ReadTransaction, ReadableDatabase, WriteTransaction};
use std::ops::Bound;
use tempfile::NamedTempFile;
use uuid::Uuid;

#[derive(Debug, Default)]
struct ServiceTagRecommender {
    index: Mutex<NlpTagIndex>,
}

impl ServiceTagRecommender {
    fn new() -> Self {
        Self::default()
    }

    fn rebuild(&self, docs: Vec<NlpDocument>) {
        self.index.lock().unwrap().build(docs);
    }

    fn recommend_tag(&self, content: &str, limit: usize) -> Vec<String> {
        self.index.lock().unwrap().recommend_tag(content, limit)
    }
}

pub struct SynapService {
    db: redb::Database,
    #[allow(dead_code)]
    tag_searcher: FuzzyIndex<Tag>,
    #[allow(dead_code)]
    note_searcher: FuzzyIndex<Note>,
    tag_recommender: ServiceTagRecommender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredNoteStatus {
    All,
    Normal,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineDirection {
    Older,
    Newer,
}

const DEFAULT_SESSION_DETECTION_CONFIG: SessionDetectionConfig =
    SessionDetectionConfig::new(5 * 60 * 1000);

#[derive(Debug, Clone, Copy)]
enum ServiceSyncRole {
    Initiator,
    Listener,
}

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
    fn parse_id(id: &str) -> Result<Uuid, ServiceError> {
        Uuid::parse_str(id).map_err(Into::into)
    }

    fn parse_ids(ids: &[String]) -> Result<Vec<Uuid>, ServiceError> {
        ids.iter().map(|id| Self::parse_id(id)).collect()
    }

    fn normalize_tag_inputs(tags: Vec<String>) -> Vec<String> {
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

    fn materialize_tags(
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

    fn rebuild_tag_search(&self) -> Result<(), ServiceError> {
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

    fn note_to_nlp_document(
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

    fn collect_tag_recommendation_docs(&self) -> Result<Vec<NlpDocument>, ServiceError> {
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

    fn rebuild_tag_recommender(&self) -> Result<(), ServiceError> {
        let docs = self.collect_tag_recommendation_docs()?;
        self.tag_recommender.rebuild(docs);
        Ok(())
    }

    fn refresh_tag_indexes(&self) -> Result<(), ServiceError> {
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
        TagWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        CryptoWriter::init_schema(&tx).map_err(|err| ServiceError::Db(err.into()))?;
        crypto::ensure_local_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        crypto::ensure_local_signing_identity(&CryptoWriter::new(&tx))
            .map_err(|err| ServiceError::Db(err.into()))?;
        tx.commit().map_err(ServiceError::CommitErr)?;

        let tag_searcher = FuzzyIndex::<Tag>::new();
        let note_searcher = FuzzyIndex::<Note>::new();
        let tag_recommender = ServiceTagRecommender::new();

        let res = Self {
            db,
            tag_searcher,
            note_searcher,
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

    pub fn get_local_identity(&self) -> Result<LocalIdentityDTO, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        let identity_public_key = crypto::local_identity_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local identity public key".into()))?;
        let signing_public_key = crypto::local_signing_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local signing public key".into()))?;

        Ok(LocalIdentityDTO {
            identity: Self::public_key_info_to_dto(
                crypto::local_identity_key_id().to_string(),
                "x25519".into(),
                identity_public_key,
            ),
            signing: Self::public_key_info_to_dto(
                crypto::local_signing_key_id().to_string(),
                "ed25519".into(),
                signing_public_key,
            ),
        })
    }

    pub fn get_peers(&self) -> Result<Vec<PeerDTO>, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        crypto::list_known_public_keys(&reader)
            .map(|records| records.into_iter().map(Self::peer_to_dto).collect())
            .map_err(Into::into)
    }

    pub fn trust_peer(
        &self,
        public_key: &[u8],
        note: Option<String>,
    ) -> Result<PeerDTO, ServiceError> {
        let public_key: [u8; 32] = public_key.try_into().map_err(|_| {
            ServiceError::Other(anyhow::anyhow!("peer public key must be 32 bytes"))
        })?;
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        let record = crypto::import_trusted_public_key(&writer, public_key, note)?;
        tx.commit()?;
        Ok(Self::peer_to_dto(record))
    }

    pub fn export_share(&self, note_ids: &[String]) -> Result<Vec<u8>, ServiceError> {
        let note_ids = Self::parse_ids(note_ids)?;
        ShareService::new(self).export_bytes(&note_ids)
    }

    pub fn import_share(&self, bytes: &[u8]) -> Result<ShareStatsDTO, ServiceError> {
        ShareService::new(self)
            .import_bytes(bytes)
            .map(Self::share_stats_to_dto)
    }

    pub fn initiate_sync<T>(&self, transport: T) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        self.run_sync_session(transport, ServiceSyncRole::Initiator)
    }

    pub fn listen_sync<T>(&self, transport: T) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        self.run_sync_session(transport, ServiceSyncRole::Listener)
    }

    fn run_sync_session<T>(
        &self,
        transport: T,
        role: ServiceSyncRole,
    ) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        let channel = {
            let tx = self.db.begin_read()?;
            let reader = CryptoReader::new(&tx)?;
            match role {
                ServiceSyncRole::Initiator => {
                    crypto::CryptoChannel::connect(transport, &reader, Default::default())
                }
                ServiceSyncRole::Listener => {
                    crypto::CryptoChannel::accept(transport, &reader, Default::default())
                }
            }
        };

        let mut channel = match channel {
            Ok(channel) => channel,
            Err(crypto::CryptoChannelError::UntrustedPeer {
                public_key,
                fingerprint: _fingerprint,
            }) => {
                let tx = self.db.begin_write()?;
                let writer = CryptoWriter::new(&tx);
                let record = crypto::remember_untrusted_public_key(&writer, public_key, None)?;
                tx.commit()?;
                return Ok(SyncSessionDTO {
                    status: SyncStatusDTO::PendingTrust,
                    peer: Self::peer_to_dto(record),
                    stats: None,
                });
            }
            Err(err) => return Err(ServiceError::Other(anyhow::anyhow!(err))),
        };

        let peer = channel.peer().clone();
        let sync_service = SyncService::new(self, Default::default());
        let stats = match role {
            ServiceSyncRole::Initiator => sync_service
                .sync_as_initiator(&mut channel)
                .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?,
            ServiceSyncRole::Listener => sync_service
                .sync_as_responder(&mut channel)
                .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?,
        };

        Ok(SyncSessionDTO {
            status: SyncStatusDTO::Completed,
            peer: Self::peer_to_dto(peer.trust_record),
            stats: Some(Self::sync_stats_to_dto(stats)),
        })
    }

    fn public_key_info_to_dto(
        id: String,
        algorithm: String,
        public_key: [u8; 32],
    ) -> PublicKeyInfoDTO {
        let fingerprint = crypto::public_key_fingerprint(&public_key);
        PublicKeyInfoDTO {
            id,
            algorithm,
            public_key: public_key.to_vec(),
            fingerprint: fingerprint.to_vec(),
            kaomoji_fingerprint: crypto::generate_kaomoji_fingerprint(&fingerprint),
        }
    }

    fn peer_to_dto(record: crypto::TrustedPublicKeyRecord) -> PeerDTO {
        let status = match record.status {
            crate::models::crypto::KeyStatus::Pending => PeerTrustStatusDTO::Pending,
            crate::models::crypto::KeyStatus::Active => PeerTrustStatusDTO::Trusted,
            crate::models::crypto::KeyStatus::Retired => PeerTrustStatusDTO::Retired,
            crate::models::crypto::KeyStatus::Revoked => PeerTrustStatusDTO::Revoked,
        };

        PeerDTO {
            id: record.id.to_string(),
            algorithm: record.algorithm,
            public_key: record.public_key.to_vec(),
            fingerprint: record.fingerprint.to_vec(),
            kaomoji_fingerprint: crypto::generate_kaomoji_fingerprint(&record.fingerprint),
            note: record.note,
            status,
        }
    }

    fn sync_stats_to_dto(stats: crate::sync::SyncStats) -> SyncStatsDTO {
        SyncStatsDTO {
            records_sent: stats.records_sent as u64,
            records_received: stats.records_received as u64,
            records_applied: stats.records_applied as u64,
            records_skipped: stats.records_skipped as u64,
            bytes_sent: stats.bytes_sent as u64,
            bytes_received: stats.bytes_received as u64,
            duration_ms: stats.duration_ms,
        }
    }

    fn share_stats_to_dto(stats: crate::sync::ShareStats) -> ShareStatsDTO {
        ShareStatsDTO {
            records: stats.records as u64,
            records_applied: stats.applied as u64,
            bytes: stats.bytes as u64,
            duration_ms: stats.duration_ms,
        }
    }

    fn init_search(&self) -> Result<(), ServiceError> {
        self.note_searcher.clear();
        self.tag_searcher.clear();

        self.with_read(|tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if Self::is_latest_version(reader, note_ref)? {
                    let note = note_ref
                        .hydrate(reader)?
                        .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                    notes.push(note);
                }
            }

            self.note_searcher.insert_batch(notes.into_iter());

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

    pub(crate) fn refresh_search_indexes(&self) -> Result<(), ServiceError> {
        self.init_search()?;
        self.rebuild_tag_recommender()
    }

    // 读操作 (Queries) - 纯惰性组装，返回 DTO
    fn note_to_dto(&self, value: Note, reader: &NoteReader<'_>) -> Result<NoteDTO, ServiceError> {
        let view = NoteView::new(reader, value);
        view.to_dto().map_err(Into::into)
    }

    fn note_ref_to_dto(
        &self,
        note_ref: NoteRef,
        reader: &NoteReader<'_>,
    ) -> Result<NoteDTO, ServiceError> {
        let note = note_ref
            .hydrate(reader)?
            .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
        self.note_to_dto(note, reader)
    }

    fn encode_timeline_cursor(note_id: Uuid) -> String {
        note_id.to_string()
    }

    fn decode_timeline_cursor(cursor: Option<&str>) -> Result<Option<Uuid>, ServiceError> {
        cursor.map(Self::parse_id).transpose()
    }

    fn finalize_note_page(mut notes: Vec<NoteDTO>, limit: usize) -> TimelineNotesPageDTO {
        let has_more = notes.len() > limit;
        if has_more {
            notes.pop();
        }

        let next_cursor = if has_more {
            notes
                .last()
                .and_then(|note| Self::parse_id(&note.id).ok())
                .map(Self::encode_timeline_cursor)
        } else {
            None
        };

        TimelineNotesPageDTO { notes, next_cursor }
    }

    fn timeline_bounds(
        direction: TimelineDirection,
        cursor: Option<Uuid>,
    ) -> (Bound<Uuid>, Bound<Uuid>, bool) {
        match (direction, cursor) {
            (TimelineDirection::Older, Some(anchor)) => {
                (Bound::Unbounded, Bound::Excluded(anchor), true)
            }
            (TimelineDirection::Newer, Some(anchor)) => {
                (Bound::Excluded(anchor), Bound::Unbounded, false)
            }
            (TimelineDirection::Older, None) => (Bound::Unbounded, Bound::Unbounded, true),
            (TimelineDirection::Newer, None) => (Bound::Unbounded, Bound::Unbounded, false),
        }
    }

    fn session_span_to_dto(
        &self,
        session: &SessionSpan,
        timeline: &TimelineView<'_>,
        reader: &NoteReader<'_>,
    ) -> Result<TimelineSessionDTO, ServiceError> {
        let notes = timeline
            .refs_in_session(session)?
            .map(|res| -> Result<NoteDTO, ServiceError> {
                let note_ref = res.map_err(ServiceError::from)?;
                self.note_ref_to_dto(note_ref, reader)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TimelineSessionDTO {
            started_at: session.started_at(),
            ended_at: session.ended_at(),
            note_count: notes.len() as u32,
            notes,
        })
    }

    fn require_note_ref(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<NoteRef, ServiceError> {
        reader
            .get_ref_by_id(&id)?
            .ok_or(ServiceError::NotFound(original.to_string()))
    }

    fn require_live_note_ref(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<NoteRef, ServiceError> {
        let note_ref = Self::require_note_ref(reader, id, original)?;
        if note_ref.is_deleted() {
            return Err(ServiceError::NotFound(original.to_string()));
        }
        Ok(note_ref)
    }

    fn require_note(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<Note, ServiceError> {
        reader
            .get_by_id(&id)?
            .ok_or(ServiceError::NotFound(original.to_string()))
    }

    fn require_live_note(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<Note, ServiceError> {
        let note = Self::require_note(reader, id, original)?;
        if note.is_deleted() {
            return Err(ServiceError::NotFound(original.to_string()));
        }
        Ok(note)
    }

    fn resolve_tag(tx: &ReadTransaction, content: &str) -> Result<Option<Tag>, ServiceError> {
        TagReader::new(tx)?
            .find_by_content(content)
            .map_err(Into::into)
    }

    fn is_latest_version(reader: &NoteReader<'_>, note_ref: NoteRef) -> Result<bool, ServiceError> {
        Ok(!reader
            .has_next_version(&note_ref.get_id())
            .map_err(redb::Error::from)?)
    }

    fn matches_selected_tags(
        note: &Note,
        selected_tag_ids: &HashSet<Uuid>,
        include_untagged: bool,
    ) -> bool {
        if note.tags().is_empty() {
            return include_untagged;
        }

        note.tags()
            .iter()
            .any(|tag_id| selected_tag_ids.contains(tag_id))
    }

    /// 获取单条笔记的完整视图
    pub fn get_note(&self, id_or_short_id: &str) -> Result<NoteDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let note = match id_or_short_id.len() {
                36_usize | 32_usize => reader.get_by_id(&Self::parse_id(id_or_short_id)?)?,
                8 => reader.get_by_short_id(id_or_short_id.as_bytes().try_into()?)?,
                _ => return Err(ServiceError::InvalidId),
            };
            let note = note.ok_or(ServiceError::NotFound(id_or_short_id.to_string()))?;
            if note.is_deleted() {
                return Err(ServiceError::NotFound(id_or_short_id.to_string()));
            }
            self.note_to_dto(note, reader)
        })
    }

    /// 获取子节点（瀑布流/无限滚动）核心接口！
    /// cursor: 前端传列表里最后一条数据的 Uuid。如果是第一次加载，传 None。
    pub fn get_replies(
        &self,
        parent_id: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(parent_id)?;
            let note = Self::require_live_note(reader, uuid, parent_id)?;
            let cursor_uuid = cursor.map(|c| Self::parse_id(&c)).transpose()?;
            let view = NoteView::new(reader, note);
            let mut children_iter = view.children_refs()?;

            if let Some(target_id) = cursor_uuid {
                // 使用 .by_ref() 借用迭代器，不断消耗元素，直到找到游标
                for res in children_iter.by_ref() {
                    let child_ref = res.map_err(ServiceError::from)?;

                    if child_ref.get_id() == target_id {
                        break;
                    }
                }
            }

            children_iter
                .take(limit)
                .map(|res| -> Result<NoteDTO, ServiceError> {
                    let child_ref = res.map_err(ServiceError::from)?;
                    self.note_ref_to_dto(child_ref, reader)
                })
                .collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_recent_notes_page(
        &self,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::with_capacity(limit.saturating_add(1));
            let note_refs: Box<dyn Iterator<Item = Result<NoteRef, crate::error::NoteError>> + '_> =
                match (direction, cursor_uuid) {
                    (TimelineDirection::Older, Some(anchor)) => {
                        let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                        split.older
                    }
                    (TimelineDirection::Newer, Some(anchor)) => {
                        let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                        split.newer
                    }
                    (TimelineDirection::Older, None) => Box::new(timeline.recent_refs()?),
                    (TimelineDirection::Newer, None) => Box::new(timeline.oldest_refs()?),
                };

            for res in note_refs {
                let note_ref = res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, note_ref)? {
                    continue;
                }

                notes.push(self.note_ref_to_dto(note_ref, reader)?);
                if notes.len() > limit {
                    break;
                }
            }

            Ok(Self::finalize_note_page(notes, limit))
        })
    }

    pub fn get_recent_note(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.get_recent_notes_page(cursor, TimelineDirection::Older, limit)
            .map(|page| page.notes)
    }

    pub fn get_recent_sessions(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<TimelineSessionsPageDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let timeline = TimelineView::new(reader);
            let session_iter = timeline.recent_session_spans(DEFAULT_SESSION_DETECTION_CONFIG)?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut spans = Vec::with_capacity(limit.saturating_add(1));

            for session in session_iter {
                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| session.cursor() == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                spans.push(session);
                if spans.len() > limit {
                    break;
                }
            }

            let has_more = spans.len() > limit;
            if has_more {
                spans.pop();
            }

            let next_cursor = if has_more {
                spans.last().map(|session| session.cursor().to_string())
            } else {
                None
            };

            let sessions = spans
                .iter()
                .map(|session| self.session_span_to_dto(session, &timeline, reader))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(TimelineSessionsPageDTO {
                sessions,
                next_cursor,
            })
        })
    }

    pub fn get_origins(&self, child_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let child_uuid = Self::parse_id(child_id)?;
            let child = Self::require_live_note(reader, child_uuid, child_id)?;
            let view = NoteView::new(reader, child);
            let parents_iter = view.parents_refs()?;

            parents_iter
                .map(|res| -> Result<NoteDTO, ServiceError> {
                    let parent_ref = res.map_err(ServiceError::from)?;
                    self.note_ref_to_dto(parent_ref, reader)
                })
                .collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_previous_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.history_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_next_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.next_version_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_other_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.other_versions_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_deleted_notes(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut notes = Vec::with_capacity(limit);
            let deleted_ids = reader.deleted_note_ids().map_err(redb::Error::from)?;

            for deleted_id in deleted_ids.rev() {
                let deleted_id = deleted_id.map_err(redb::Error::from)?;

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| deleted_id == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                let note_ref = Self::require_note_ref(reader, deleted_id, &deleted_id.to_string())?;
                notes.push(self.note_ref_to_dto(note_ref, reader)?);
                if notes.len() == limit {
                    break;
                }
            }

            Ok(notes)
        })
    }

    /// 横向检索
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<NoteDTO>, ServiceError> {
        let search_res = self.note_searcher.search(query, limit, None);
        let uuids = search_res.items;
        self.with_read(|_tx, reader| {
            uuids
                .iter()
                .map(|id| reader.get_by_id(&id.id)?.ok_or(ServiceError::InvalidId))
                .map(|note| self.note_to_dto(note?, reader))
                .collect()
        })
    }

    pub fn search_tags(&self, query: &str, limit: usize) -> Result<Vec<String>, ServiceError> {
        let search_res = self.tag_searcher.search(query, limit, None);
        let ids = search_res.items;

        self.with_read(|tx, _reader| {
            let tag_reader = TagReader::new(tx)?;
            let mut seen = HashSet::new();

            ids.into_iter()
                .filter_map(|item| match tag_reader.get_by_id(&item.id) {
                    Ok(Some(tag)) => {
                        let content = tag.get_content().to_string();
                        seen.insert(content.clone()).then_some(Ok(content))
                    }
                    Ok(None) => None,
                    Err(err) => Some(Err(ServiceError::Db(err))),
                })
                .collect()
        })
    }

    pub fn recommend_tag(&self, content: &str, limit: usize) -> Result<Vec<String>, ServiceError> {
        Ok(self.tag_recommender.recommend_tag(content, limit))
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>, ServiceError> {
        self.with_read(|tx, _reader| {
            let tag_reader = TagReader::new(tx)?;
            let mut tags = tag_reader
                .all()
                .map_err(redb::Error::from)?
                .map(|tag| tag.map(|tag| tag.get_content().to_string()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;

            tags.sort();
            Ok(tags)
        })
    }

    /// tag-centric 的便捷查询。
    ///
    /// 它复用的是 `tag -> note` 关系，因此更适合标签页、标签搜索结果等
    /// “从标签出发”的场景；如果需求是首页/时间流那种“全局时间轴 + 游标”
    /// 过滤，请走 `get_filtered_notes`，不要和这里混用。
    pub fn get_notes_by_tag(
        &self,
        tag: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let Some(tag) = Self::resolve_tag(tx, tag)? else {
                return Ok(Vec::new());
            };

            let tagged_iter = reader.latest_notes_with_tag(&tag)?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut notes = Vec::with_capacity(limit);

            for note in tagged_iter {
                let note = note?;

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| note.get_id() == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                notes.push(self.note_to_dto(note, reader)?);
                if notes.len() == limit {
                    break;
                }
            }

            Ok(notes)
        })
    }

    /// 时间轴过滤页。
    ///
    /// 新接口返回服务端维护的分页 token，而不是要求调用方自己记最后一条
    /// note 的 id。对于 normal feed，会优先走时间轴的双向切分能力；
    /// 对 deleted/all 这类不完全等价于 live timeline 的视图，则退回到
    /// 基于时间范围的原始扫描。
    pub fn get_filtered_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        let limit = limit.unwrap_or(20);

        if !tag_filter_enabled {
            return match status {
                FilteredNoteStatus::Normal => {
                    self.get_recent_notes_page(cursor, direction, Some(limit))
                }
                FilteredNoteStatus::Deleted => self.with_read(|_tx, reader| {
                    let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let deleted_ids: Box<
                        dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_,
                    > = if reverse {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?
                                .rev(),
                        )
                    } else {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?,
                        )
                    };
                    let mut notes = Vec::with_capacity(limit.saturating_add(1));

                    for note_id in deleted_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        notes.push(self.note_ref_to_dto(note_ref, reader)?);
                        if notes.len() > limit {
                            break;
                        }
                    }

                    Ok(Self::finalize_note_page(notes, limit))
                }),
                FilteredNoteStatus::All => self.with_read(|_tx, reader| {
                    let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let note_ids: Box<dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_> =
                        if reverse {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?
                                    .rev(),
                            )
                        } else {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?,
                            )
                        };
                    let mut notes = Vec::with_capacity(limit.saturating_add(1));

                    for note_id in note_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;

                        if !Self::is_latest_version(reader, note_ref)? {
                            continue;
                        }

                        notes.push(self.note_ref_to_dto(note_ref, reader)?);
                        if notes.len() > limit {
                            break;
                        }
                    }

                    Ok(Self::finalize_note_page(notes, limit))
                }),
            };
        }

        let selected_tag_ids = Self::normalize_tag_inputs(selected_tags)
            .into_iter()
            .filter_map(|tag| Tag::id_for_content(&tag))
            .collect::<HashSet<_>>();

        if selected_tag_ids.is_empty() && !include_untagged {
            return Ok(TimelineNotesPageDTO {
                notes: Vec::new(),
                next_cursor: None,
            });
        }

        self.with_read(|_tx, reader| {
            let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
            let mut notes = Vec::with_capacity(limit.saturating_add(1));

            let mut maybe_push = |note_ref: NoteRef| -> Result<bool, ServiceError> {
                if !Self::is_latest_version(reader, note_ref)? {
                    return Ok(false);
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;

                if !Self::matches_selected_tags(&note, &selected_tag_ids, include_untagged) {
                    return Ok(false);
                }

                notes.push(self.note_to_dto(note, reader)?);
                Ok(notes.len() > limit)
            };

            match status {
                FilteredNoteStatus::Normal => {
                    let timeline = TimelineView::new(reader);
                    let note_refs: Box<
                        dyn Iterator<Item = Result<NoteRef, crate::error::NoteError>> + '_,
                    > = match (direction, cursor_uuid) {
                        (TimelineDirection::Older, Some(anchor)) => {
                            let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                            split.older
                        }
                        (TimelineDirection::Newer, Some(anchor)) => {
                            let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                            split.newer
                        }
                        (TimelineDirection::Older, None) => Box::new(timeline.recent_refs()?),
                        (TimelineDirection::Newer, None) => Box::new(timeline.oldest_refs()?),
                    };

                    for note_ref in note_refs {
                        if maybe_push(note_ref.map_err(ServiceError::from)?)? {
                            break;
                        }
                    }
                }
                FilteredNoteStatus::Deleted => {
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let deleted_ids: Box<
                        dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_,
                    > = if reverse {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?
                                .rev(),
                        )
                    } else {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?,
                        )
                    };

                    for note_id in deleted_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        if maybe_push(note_ref)? {
                            break;
                        }
                    }
                }
                FilteredNoteStatus::All => {
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let note_ids: Box<dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_> =
                        if reverse {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?
                                    .rev(),
                            )
                        } else {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?,
                            )
                        };

                    for note_id in note_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        if maybe_push(note_ref)? {
                            break;
                        }
                    }
                }
            }

            Ok(Self::finalize_note_page(notes, limit))
        })
    }

    /// 时间轴过滤的唯一入口。
    ///
    /// 兼容旧的“只向旧内容翻页”调用；新的分页 token 入口请走
    /// `get_filtered_notes_page`。
    pub fn get_filtered_notes(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.get_filtered_notes_page(
            selected_tags,
            include_untagged,
            tag_filter_enabled,
            status,
            cursor,
            TimelineDirection::Older,
            limit,
        )
        .map(|page| page.notes)
    }

    // ------------------------------------------
    // 写操作 (Commands) - 消费输入，改变世界，返回最新 DTO
    // ------------------------------------------

    pub fn create_note(&self, content: String, tags: Vec<String>) -> Result<NoteDTO, ServiceError> {
        let note = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            Note::create(tx, content, tags).map_err(Into::into)
        })?;

        self.note_searcher.insert(note.clone());
        self.refresh_tag_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(note.clone(), reader))
    }

    pub fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let parent_id = Self::parse_id(parent_id)?;
        let parent_ref = self.with_read(|_tx, reader| {
            Self::require_live_note_ref(reader, parent_id, &parent_id.to_string())
        })?;

        let child = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            let child = Note::create(tx, content, tags)?;
            parent_ref.reply_to(tx, &child.get_id())?;
            Ok(child)
        })?;

        self.note_searcher.insert(child.clone());
        self.refresh_tag_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(child.clone(), reader))
    }

    /// 进化操作
    pub fn edit_note(
        &self,
        target_id: &str,
        new_content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let target_id = Self::parse_id(target_id)?;
        let note_ref = self.with_read(|_tx, reader| {
            Self::require_live_note_ref(reader, target_id, &target_id.to_string())
        })?;

        let edited = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            note_ref.edit(tx, new_content, tags).map_err(Into::into)
        })?;

        self.refresh_search_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(edited.clone(), reader))
    }

    /// 召唤死神
    pub fn delete_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.del(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }

    pub fn restore_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.restore(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn seed_db(path: &Path, tags: &[&str]) {
        let db = Database::create(path).unwrap();

        let tx = db.begin_write().unwrap();
        Note::init_schema(&tx).unwrap();
        TagWriter::init_schema(&tx).unwrap();

        let tag_writer = TagWriter::new(&tx);
        for tag in tags {
            tag_writer.find_or_create(*tag).unwrap();
        }

        tx.commit().unwrap();
    }

    #[test]
    fn test_search_tags_uses_initialized_tag_index() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        seed_db(&db_path, &["rust", "python", "async-rust"]);

        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let results = service.search_tags("rust", 10).unwrap();
        assert!(results.iter().any(|tag| tag == "rust"));
        assert!(results.iter().any(|tag| tag == "async-rust"));
        assert!(!results.iter().any(|tag| tag == "python"));
    }

    #[test]
    fn test_open_existing_db_auto_creates_crypto_schema_and_identity() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        seed_db(&db_path, &["rust"]);

        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
        let local_identity = service.get_local_identity().unwrap();
        assert_eq!(local_identity.identity.public_key.len(), 32);
        assert_eq!(local_identity.signing.public_key.len(), 32);
        assert!(!local_identity.identity.kaomoji_fingerprint.is_empty());
        assert!(!local_identity.signing.kaomoji_fingerprint.is_empty());

        drop(service);

        let reopened = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
        let reopened_identity = reopened.get_local_identity().unwrap();
        assert_eq!(reopened_identity.identity.public_key, local_identity.identity.public_key);
        assert_eq!(reopened_identity.signing.public_key, local_identity.signing.public_key);
    }

    #[test]
    fn test_recommend_tag_returns_related_tags() {
        let service = SynapService::new(None).unwrap();

        service
            .create_note(
                "Rust ownership and lifetimes for async services".to_string(),
                vec!["rust".into(), "async".into(), "backend".into()],
            )
            .unwrap();
        service
            .create_note(
                "Tokio runtime, future polling and async scheduling".to_string(),
                vec!["rust".into(), "async".into()],
            )
            .unwrap();
        service
            .create_note(
                "数据库索引与查询优化实践".to_string(),
                vec!["database".into(), "backend".into()],
            )
            .unwrap();

        let tags = service.recommend_tag("tokio async ownership", 3).unwrap();
        assert!(tags.iter().any(|tag| tag == "rust"));
        assert!(tags.iter().any(|tag| tag == "async"));
    }

    #[test]
    fn test_recommend_tag_tracks_note_lifecycle() {
        let service = SynapService::new(None).unwrap();

        let original = service
            .create_note("tokio future runtime".to_string(), vec!["async".into()])
            .unwrap();

        let initial = service.recommend_tag("tokio runtime", 3).unwrap();
        assert!(initial.iter().any(|tag| tag == "async"));

        let edited = service
            .edit_note(
                &original.id,
                "sql index join planner".to_string(),
                vec!["database".into()],
            )
            .unwrap();

        let updated = service.recommend_tag("sql planner", 3).unwrap();
        assert!(updated.iter().any(|tag| tag == "database"));

        let old_query = service.recommend_tag("tokio runtime", 3).unwrap();
        assert!(!old_query.iter().any(|tag| tag == "async"));

        service.delete_note(&edited.id).unwrap();
        let after_delete = service.recommend_tag("sql planner", 3).unwrap();
        assert!(!after_delete.iter().any(|tag| tag == "database"));

        service.restore_note(&edited.id).unwrap();
        let after_restore = service.recommend_tag("sql planner", 3).unwrap();
        assert!(after_restore.iter().any(|tag| tag == "database"));
    }

    #[test]
    fn test_get_all_tags_returns_sorted_contents() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        service
            .create_note(
                "tagged".to_string(),
                vec![
                    " rust ".into(),
                    "async".into(),
                    "python".into(),
                    "rust".into(),
                ],
            )
            .unwrap();

        let tags = service.get_all_tags().unwrap();
        assert_eq!(
            tags,
            vec![
                "async".to_string(),
                "python".to_string(),
                "rust".to_string(),
            ]
        );
    }

    #[test]
    fn test_get_notes_by_tag_returns_only_live_latest_matches() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let dropped = service
            .create_note("learn rust".to_string(), vec!["rust".into()])
            .unwrap();
        let _replacement = service
            .edit_note(&dropped.id, "learn async".to_string(), vec!["async".into()])
            .unwrap();

        let deleted = service
            .create_note("ship rust".to_string(), vec!["rust".into()])
            .unwrap();
        service.delete_note(&deleted.id).unwrap();

        let live = service
            .create_note("keep rust".to_string(), vec!["rust".into()])
            .unwrap();

        let rust_notes = service.get_notes_by_tag(" rust ", None, None).unwrap();
        assert_eq!(rust_notes.len(), 1);
        assert_eq!(rust_notes[0].id, live.id);

        assert!(service
            .get_notes_by_tag("missing", None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_get_notes_by_tag_uses_cursor_pagination() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let first = service
            .create_note("rust first".to_string(), vec!["rust".into()])
            .unwrap();
        let second = service
            .create_note("rust second".to_string(), vec!["rust".into()])
            .unwrap();
        let third = service
            .create_note("rust third".to_string(), vec!["rust".into()])
            .unwrap();

        let page_one = service.get_notes_by_tag("rust", None, Some(2)).unwrap();
        assert_eq!(page_one.len(), 2);
        assert_eq!(page_one[0].id, first.id);
        assert_eq!(page_one[1].id, second.id);

        let page_two = service
            .get_notes_by_tag("rust", Some(&page_one[1].id), Some(2))
            .unwrap();
        assert_eq!(page_two.len(), 1);
        assert_eq!(page_two[0].id, third.id);
    }

    #[test]
    fn test_get_filtered_notes_keeps_global_time_order() {
        let service = SynapService::new(None).unwrap();

        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();
        let fourth = service.create_note("fourth".to_string(), vec![]).unwrap();

        service.delete_note(&second.id).unwrap();
        service.delete_note(&fourth.id).unwrap();

        let filtered = service
            .get_filtered_notes(vec![], true, false, FilteredNoteStatus::All, None, Some(10))
            .unwrap();

        assert_eq!(
            filtered
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![fourth.id, third.id, second.id, first.id]
        );
    }

    #[test]
    fn test_get_filtered_notes_supports_mixed_tags_and_untagged() {
        let service = SynapService::new(None).unwrap();

        let rust = service
            .create_note("rust".to_string(), vec!["rust".into()])
            .unwrap();
        let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
        let travel = service
            .create_note("travel".to_string(), vec!["travel".into()])
            .unwrap();
        let rust_work = service
            .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
            .unwrap();

        let filtered = service
            .get_filtered_notes(
                vec!["rust".into(), "travel".into()],
                true,
                true,
                FilteredNoteStatus::Normal,
                None,
                Some(10),
            )
            .unwrap();

        assert_eq!(
            filtered
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![rust_work.id, travel.id, untagged.id, rust.id]
        );
    }

    #[test]
    fn test_get_filtered_notes_uses_cursor_after_filtering() {
        let service = SynapService::new(None).unwrap();

        let rust = service
            .create_note("rust".to_string(), vec!["rust".into()])
            .unwrap();
        let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
        let travel = service
            .create_note("travel".to_string(), vec!["travel".into()])
            .unwrap();
        let rust_work = service
            .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
            .unwrap();

        let page_one = service
            .get_filtered_notes(
                vec!["rust".into(), "travel".into()],
                true,
                true,
                FilteredNoteStatus::Normal,
                None,
                Some(2),
            )
            .unwrap();
        assert_eq!(page_one.len(), 2);
        assert_eq!(page_one[0].id, rust_work.id);
        assert_eq!(page_one[1].id, travel.id);

        let page_two = service
            .get_filtered_notes(
                vec!["rust".into(), "travel".into()],
                true,
                true,
                FilteredNoteStatus::Normal,
                Some(&page_one[1].id),
                Some(2),
            )
            .unwrap();
        assert_eq!(page_two.len(), 2);
        assert_eq!(page_two[0].id, untagged.id);
        assert_eq!(page_two[1].id, rust.id);
    }

    #[test]
    fn test_create_note_updates_service_searchers() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let created = service
            .create_note(
                "learn rust ownership".to_string(),
                vec![" rust ".into(), "async".into(), "rust".into(), "".into()],
            )
            .unwrap();

        assert_eq!(created.content, "learn rust ownership");
        assert_eq!(created.tags, vec!["rust".to_string(), "async".to_string()]);

        let note_hits = service.search("ownership", 10).unwrap();
        assert!(note_hits.iter().any(|note| note.id == created.id));

        let tag_hits = service.search_tags("rust", 10).unwrap();
        assert!(tag_hits.iter().any(|tag| tag == "rust"));
    }

    #[test]
    fn test_create_note_exposes_millisecond_timestamp() {
        let service = SynapService::new(None).unwrap();
        let created = service.create_note("timed".to_string(), vec![]).unwrap();

        assert!(created.created_at >= 1_000_000_000_000);
    }

    #[test]
    fn test_edit_note_creates_new_version_and_refreshes_tags() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let created = service
            .create_note("learn rust".to_string(), vec!["rust".into()])
            .unwrap();

        let edited = service
            .edit_note(
                &created.id,
                "learn rust async".to_string(),
                vec!["rust".into(), "async".into()],
            )
            .unwrap();

        assert_ne!(created.id, edited.id);
        assert_eq!(edited.content, "learn rust async");
        assert_eq!(edited.tags, vec!["rust".to_string(), "async".to_string()]);

        let tag_hits = service.search_tags("async", 10).unwrap();
        assert!(tag_hits.iter().any(|tag| tag == "async"));

        let note_hits = service.search("rust async", 10).unwrap();
        assert!(note_hits.iter().any(|note| note.id == edited.id));
    }

    #[test]
    fn test_reply_note_links_child_and_indexes_tags() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let parent = service
            .create_note("parent".to_string(), vec!["root".into()])
            .unwrap();
        let child = service
            .reply_note(&parent.id, "child".to_string(), vec!["reply".into()])
            .unwrap();

        let replies = service.get_replies(&parent.id, None, 10).unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].id, child.id);

        let tag_hits = service.search_tags("reply", 10).unwrap();
        assert!(tag_hits.iter().any(|tag| tag == "reply"));
    }

    #[test]
    fn test_get_recent_note_uses_cursor_pagination() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();

        let page_one = service.get_recent_note(None, Some(2)).unwrap();
        assert_eq!(page_one.len(), 2);
        assert_eq!(page_one[0].id, third.id);
        assert_eq!(page_one[1].id, second.id);

        let page_two = service
            .get_recent_note(Some(&page_one[1].id), Some(2))
            .unwrap();
        assert_eq!(page_two.len(), 1);
        assert_eq!(page_two[0].id, first.id);
    }

    #[test]
    fn test_get_recent_notes_page_returns_service_cursor() {
        let service = SynapService::new(None).unwrap();

        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();

        let page_one = service
            .get_recent_notes_page(None, TimelineDirection::Older, Some(2))
            .unwrap();

        assert_eq!(
            page_one
                .notes
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![third.id.clone(), second.id.clone()]
        );
        assert_eq!(page_one.next_cursor.as_deref(), Some(second.id.as_str()));

        let page_two = service
            .get_recent_notes_page(
                page_one.next_cursor.as_deref(),
                TimelineDirection::Older,
                Some(2),
            )
            .unwrap();

        assert_eq!(page_two.notes.len(), 1);
        assert_eq!(page_two.notes[0].id, first.id);
        assert!(page_two.next_cursor.is_none());
    }

    #[test]
    fn test_get_filtered_notes_page_uses_service_cursor_after_filtering() {
        let service = SynapService::new(None).unwrap();

        let rust = service
            .create_note("rust".to_string(), vec!["rust".into()])
            .unwrap();
        let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
        let travel = service
            .create_note("travel".to_string(), vec!["travel".into()])
            .unwrap();
        let rust_work = service
            .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
            .unwrap();

        let page_one = service
            .get_filtered_notes_page(
                vec!["rust".into(), "travel".into()],
                true,
                true,
                FilteredNoteStatus::Normal,
                None,
                TimelineDirection::Older,
                Some(2),
            )
            .unwrap();

        assert_eq!(
            page_one
                .notes
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![rust_work.id.clone(), travel.id.clone()]
        );
        assert_eq!(page_one.next_cursor.as_deref(), Some(travel.id.as_str()));

        let page_two = service
            .get_filtered_notes_page(
                vec!["rust".into(), "travel".into()],
                true,
                true,
                FilteredNoteStatus::Normal,
                page_one.next_cursor.as_deref(),
                TimelineDirection::Older,
                Some(2),
            )
            .unwrap();

        assert_eq!(
            page_two
                .notes
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![untagged.id, rust.id]
        );
        assert!(page_two.next_cursor.is_none());
    }

    #[test]
    fn test_get_recent_sessions_returns_hydrated_notes() {
        let service = SynapService::new(None).unwrap();

        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();

        let page = service.get_recent_sessions(None, Some(10)).unwrap();

        assert!(page.next_cursor.is_none());
        assert_eq!(page.sessions.len(), 1);
        assert_eq!(page.sessions[0].note_count, 3);
        assert_eq!(
            page.sessions[0]
                .notes
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![third.id, second.id, first.id]
        );
    }

    #[test]
    fn test_get_recent_sessions_filters_deleted_and_superseded_notes() {
        let service = SynapService::new(None).unwrap();

        let original = service.create_note("draft".to_string(), vec![]).unwrap();
        let edited = service
            .edit_note(&original.id, "published".to_string(), vec![])
            .unwrap();
        let deleted = service.create_note("deleted".to_string(), vec![]).unwrap();
        service.delete_note(&deleted.id).unwrap();
        let live = service.create_note("live".to_string(), vec![]).unwrap();

        let page = service.get_recent_sessions(None, Some(10)).unwrap();
        let notes = &page.sessions[0].notes;

        assert_eq!(page.sessions.len(), 1);
        assert_eq!(page.sessions[0].note_count, 2);
        assert!(notes.iter().any(|note| note.id == edited.id));
        assert!(notes.iter().any(|note| note.id == live.id));
        assert!(!notes.iter().any(|note| note.id == original.id));
        assert!(!notes.iter().any(|note| note.id == deleted.id));
    }

    #[test]
    fn test_get_origins_returns_only_parent_layer() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let root = service.create_note("root".to_string(), vec![]).unwrap();
        let middle = service
            .reply_note(&root.id, "middle".to_string(), vec![])
            .unwrap();
        let leaf = service
            .reply_note(&middle.id, "leaf".to_string(), vec![])
            .unwrap();

        let origins = service.get_origins(&leaf.id).unwrap();
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].id, middle.id);
    }

    #[test]
    fn test_get_origins_depth_one_keeps_only_compacted_parent_layer() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let root = service.create_note("root".to_string(), vec![]).unwrap();
        let middle = service
            .reply_note(&root.id, "middle".to_string(), vec![])
            .unwrap();
        let leaf = service
            .reply_note(&middle.id, "leaf".to_string(), vec![])
            .unwrap();

        let origins = service.get_origins(&leaf.id).unwrap();
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].id, middle.id);
        assert_ne!(origins[0].id, root.id);
    }

    #[test]
    fn test_version_queries_return_live_related_versions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let v1 = service
            .create_note("Version 1".to_string(), vec![])
            .unwrap();
        let v2a = service
            .edit_note(&v1.id, "Version 2A".to_string(), vec![])
            .unwrap();
        let v2b = service
            .edit_note(&v1.id, "Version 2B".to_string(), vec![])
            .unwrap();

        let previous = service.get_previous_versions(&v2a.id).unwrap();
        assert_eq!(previous.len(), 1);
        assert_eq!(previous[0].id, v1.id);

        let next = service.get_next_versions(&v1.id).unwrap();
        assert_eq!(next.len(), 2);
        assert!(next.iter().any(|note| note.id == v2a.id));
        assert!(next.iter().any(|note| note.id == v2b.id));

        let others = service.get_other_versions(&v2a.id).unwrap();
        assert_eq!(others.len(), 2);
        assert!(others.iter().any(|note| note.id == v1.id));
        assert!(others.iter().any(|note| note.id == v2b.id));
    }

    #[test]
    fn test_deleted_note_iteration_and_restore_round_trip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();

        service.delete_note(&first.id).unwrap();
        service.delete_note(&second.id).unwrap();

        assert!(matches!(
            service.get_note(&second.id),
            Err(ServiceError::NotFound(_))
        ));

        let deleted = service.get_deleted_notes(None, Some(2)).unwrap();
        assert_eq!(deleted.len(), 2);
        assert_eq!(deleted[0].id, second.id);
        assert_eq!(deleted[1].id, first.id);

        let deleted_page_two = service
            .get_deleted_notes(Some(&deleted[0].id), Some(2))
            .unwrap();
        assert_eq!(deleted_page_two.len(), 1);
        assert_eq!(deleted_page_two[0].id, first.id);

        service.restore_note(&second.id).unwrap();

        let remaining_deleted = service.get_deleted_notes(None, Some(10)).unwrap();
        assert_eq!(remaining_deleted.len(), 1);
        assert_eq!(remaining_deleted[0].id, first.id);

        let restored = service.get_note(&second.id).unwrap();
        assert_eq!(restored.id, second.id);
    }

    #[test]
    fn test_recent_and_search_filter_superseded_versions_and_markdown_media() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("synap.redb");
        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let original = service
            .create_note(
                "hello ![cover](data:image/png;base64,AAAA) rust".to_string(),
                vec![],
            )
            .unwrap();
        let edited = service
            .edit_note(
                &original.id,
                "hello ![cover](data:image/png;base64,BBBB) rust async".to_string(),
                vec![],
            )
            .unwrap();

        let recent = service.get_recent_note(None, Some(10)).unwrap();
        assert!(recent.iter().any(|note| note.id == edited.id));
        assert!(!recent.iter().any(|note| note.id == original.id));

        let rust_hits = service.search("rust", 10).unwrap();
        assert!(rust_hits.iter().any(|note| note.id == edited.id));
        assert!(!rust_hits.iter().any(|note| note.id == original.id));

        let image_hits = service.search("AAAA", 10).unwrap();
        assert!(image_hits.is_empty());
    }

    #[test]
    fn test_share_export_and_import_are_exposed_via_service() {
        let dir = tempdir().unwrap();
        let path_a = dir.path().join("share-service-a.redb");
        let path_b = dir.path().join("share-service-b.redb");

        let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
        let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

        let root = service_a
            .create_note("share root".to_string(), vec!["rust".into()])
            .unwrap();
        let reply = service_a
            .reply_note(&root.id, "share child".to_string(), vec!["thread".into()])
            .unwrap();

        let exported = service_a
            .export_share(&vec![root.id.clone(), reply.id.clone()])
            .unwrap();
        assert!(!exported.is_empty());

        let stats = service_b.import_share(&exported).unwrap();
        assert_eq!(stats.records, 2);
        assert_eq!(stats.records_applied, 2);
        assert_eq!(stats.bytes, exported.len() as u64);

        let imported_root = service_b.get_note(&root.id).unwrap();
        let imported_reply = service_b.get_note(&reply.id).unwrap();
        assert_eq!(imported_root.content, "share root");
        assert_eq!(imported_reply.content, "share child");
    }

    #[test]
    fn test_export_share_rejects_invalid_note_ids() {
        let service = SynapService::new(None).unwrap();

        let err = service
            .export_share(&vec!["bad-id".to_string()])
            .unwrap_err();
        assert!(matches!(
            err,
            ServiceError::InvalidId | ServiceError::UuidErr(_)
        ));
    }
}
