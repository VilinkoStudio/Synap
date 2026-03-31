use std::{collections::HashSet, path::Path};

use crate::{
    dto::NoteDTO,
    error::{NoteError, ServiceError},
    models::{
        note::{Note, NoteReader},
        tag::{Tag, TagReader, TagWriter},
    },
    search::searcher::FuzzyIndex,
    views::{note_view::NoteView, timeline_view::TimelineView},
};

use redb::{Database, ReadTransaction, ReadableDatabase, WriteTransaction};
use tempfile::NamedTempFile;
use uuid::Uuid;

pub struct SynapService {
    db: redb::Database,
    tag_searcher: FuzzyIndex<Tag>,
    note_searcher: FuzzyIndex<Note>,
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
        tx.commit().map_err(ServiceError::CommitErr)?;

        let tag_searcher = FuzzyIndex::<Tag>::new();
        let note_searcher = FuzzyIndex::<Note>::new();

        let res = Self {
            db,
            tag_searcher,
            note_searcher,
        };
        res.init_search()?;
        Ok(res)
    }

    fn init_search(&self) -> Result<(), ServiceError> {
        self.note_searcher.clear();
        self.tag_searcher.clear();

        self.with_read(|tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::new();

            for view_res in timeline.recent()? {
                let view = view_res.map_err(ServiceError::from)?;
                if Self::is_latest_version(reader, view.get_note())? {
                    notes.push(view.get_note().clone());
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
        self.init_search()
    }

    // 读操作 (Queries) - 纯惰性组装，返回 DTO
    fn note_to_dto(&self, value: Note, reader: &NoteReader<'_>) -> Result<NoteDTO, ServiceError> {
        let view = NoteView::new(reader, value);
        view.to_dto().map_err(Into::into)
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

    fn is_latest_version(reader: &NoteReader<'_>, note: &Note) -> Result<bool, ServiceError> {
        let mut next_versions = reader.next_versions(note).map_err(redb::Error::from)?;
        Ok(next_versions
            .next()
            .transpose()
            .map_err(redb::Error::from)?
            .is_none())
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
            let mut children_iter = view.children()?;

            if let Some(target_id) = cursor_uuid {
                // 使用 .by_ref() 借用迭代器，不断消耗元素，直到找到游标
                for res in children_iter.by_ref() {
                    let child_view = res.map_err(|e| ServiceError::NoteErr(e))?;

                    if child_view.get_note().get_id() == target_id {
                        break;
                    }
                }
            }

            children_iter
                .take(limit)
                .map(|res| -> Result<NoteDTO, ServiceError> {
                    let child_view = res.map_err(|e| ServiceError::NoteErr(e))?;
                    child_view.to_dto().map_err(Into::into)
                })
                .collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_recent_note(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let timeline = TimelineView::new(reader);
            let recent_iter = timeline.recent()?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut notes = Vec::with_capacity(limit);

            for res in recent_iter {
                let view = res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, view.get_note())? {
                    continue;
                }

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| view.get_note().get_id() == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                notes.push(view.to_dto().map_err(ServiceError::from)?);
                if notes.len() == limit {
                    break;
                }
            }

            Ok(notes)
        })
    }

    //TODO: 所有迭代操作去除skip while 尝试能够直接跳转进度从而极限优化性能
    pub fn get_origins(&self, child_id: &str, depth: usize) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            if depth == 0 {
                return Ok(Vec::new());
            }

            let child_uuid = Self::parse_id(child_id)?;
            let child = Self::require_live_note(reader, child_uuid, child_id)?;

            let mut visited = HashSet::new();
            let mut frontier = vec![child];
            let mut origins = Vec::new();

            visited.insert(child_uuid);

            for _ in 0..depth {
                if frontier.is_empty() {
                    break;
                }

                let mut next_frontier = Vec::new();

                for note in frontier {
                    let view = NoteView::new(reader, note);
                    let parents = view.parents()?;

                    for parent_res in parents {
                        let parent_view = parent_res.map_err(ServiceError::from)?;
                        let parent_id = parent_view.get_note().get_id();
                        if !visited.insert(parent_id) {
                            continue;
                        }

                        let parent_note = parent_view.get_note().clone();

                        origins.push(self.note_to_dto(parent_note.clone(), reader)?);
                        next_frontier.push(parent_note);
                    }
                }

                frontier = next_frontier;
            }

            Ok(origins)
        })
    }

    pub fn get_previous_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.history()?;
            let mut results = Vec::new();

            for res in versions {
                let version = res.map_err(ServiceError::from)?;
                results.push(version.to_dto().map_err(ServiceError::from)?);
            }

            Ok(results)
        })
    }

    pub fn get_next_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.next_version()?;
            let mut results = Vec::new();

            for res in versions {
                let version = res.map_err(ServiceError::from)?;
                results.push(version.to_dto().map_err(ServiceError::from)?);
            }

            Ok(results)
        })
    }

    pub fn get_other_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.other_versions()?;
            let mut results = Vec::new();

            for res in versions {
                let version = res.map_err(ServiceError::from)?;
                results.push(version.to_dto().map_err(ServiceError::from)?);
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
                let note = Self::require_note(reader, deleted_id, &deleted_id.to_string())?;

                if !note.is_deleted() {
                    continue;
                }

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| deleted_id == *target_id)
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
            let mut tags = Vec::new();

            for item in ids {
                match tag_reader.get_by_id(&item.id) {
                    Ok(Some(tag)) => {
                        let content = tag.get_content().to_string();
                        if seen.insert(content.clone()) {
                            tags.push(content);
                        }
                    }
                    Ok(None) => {}
                    Err(err) => return Err(ServiceError::Db(err)),
                }
            }

            Ok(tags)
        })
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
        self.rebuild_tag_search()?;

        self.with_read(|_tx, reader| self.note_to_dto(note.clone(), reader))
    }

    pub fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let parent_id = Self::parse_id(parent_id)?;
        let parent = self.with_read(|_tx, reader| {
            let note = reader
                .get_by_id(&parent_id)?
                .ok_or(ServiceError::NotFound(parent_id.to_string()))?;

            if note.is_deleted() {
                return Err(ServiceError::NotFound(parent_id.to_string()));
            }

            Ok(note)
        })?;

        let child = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            let child = Note::create(tx, content, tags)?;
            parent.reply(tx, &child)?;
            Ok(child)
        })?;

        self.note_searcher.insert(child.clone());
        self.rebuild_tag_search()?;

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
        let note = self.with_read(|_tx, reader| {
            let note = reader
                .get_by_id(&target_id)?
                .ok_or(ServiceError::NotFound(target_id.to_string()))?;

            if note.is_deleted() {
                return Err(ServiceError::NotFound(target_id.to_string()));
            }

            Ok(note)
        })?;

        let edited = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            note.edit(tx, new_content, tags).map_err(Into::into)
        })?;

        self.refresh_search_indexes()?;
        self.rebuild_tag_search()?;

        self.with_read(|_tx, reader| self.note_to_dto(edited.clone(), reader))
    }

    /// 召唤死神
    pub fn delete_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        // 先读取获取 note，然后在写事务中删除
        let note =
            self.with_read(|_tx, reader| reader.get_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note.del(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }

    pub fn restore_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note =
            self.with_read(|_tx, reader| reader.get_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note.restore(tx)?;
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
    fn test_get_origins_walks_parent_chain_up_to_depth() {
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

        let shallow = service.get_origins(&leaf.id, 1).unwrap();
        assert_eq!(shallow.len(), 1);
        assert_eq!(shallow[0].id, middle.id);

        let deep = service.get_origins(&leaf.id, 2).unwrap();
        assert_eq!(deep.len(), 2);
        assert_eq!(deep[0].id, middle.id);
        assert_eq!(deep[1].id, root.id);
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

        let origins = service.get_origins(&leaf.id, 1).unwrap();
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
}
