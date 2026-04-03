use std::collections::{HashSet, VecDeque};

use redb::{ReadTransaction, ReadableTable, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{
        dagstorage::{DagReader, DagStore},
        kvstore::{KvReader, KvStore},
        onetomany::{OneToMany, OneToManyReader},
        setstore::{SetReader, SetStore},
        types::BlockId,
        vector::VectorStore,
    },
    error::NoteError,
    models::{tag::Tag, util::random_id},
    search::types::Searchable,
    text::sanitize_search_text,
};

// ==========================================
// 数据层
// ==========================================

#[derive(Serialize, Deserialize, Clone)]
struct NoteBlock {
    pub content: String,
    pub short_id: [u8; 8],
    pub tags: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteSyncRecord {
    pub id: Uuid,
    pub content: String,
    pub short_id: [u8; 8],
    pub tags: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteRef {
    id: Uuid,
    deleted: bool,
}

/// Note：统一的数据结构，Reader 返回它，Writer 消费它
#[derive(Clone)]
pub struct Note {
    id: Uuid,
    deleted: bool,
    inner: NoteBlock,
}

// 静态表定义
const NOTE_STORE: KvStore<BlockId, NoteBlock> = KvStore::new("NoteBlocks");
const ID_ALIAS: KvStore<[u8; 8], BlockId> = KvStore::new("IdAlias");
const NOTE_EDIT: DagStore = DagStore::new("NoteEditForward", "NoteEditRev");
const NOTE_LINK: DagStore = DagStore::new("NoteLinkForward", "NoteLinkRev");
const NOTE_DELETE: SetStore<BlockId> = SetStore::new("NoteDeleted");
// 可重建的二级索引：只追加 tag -> note 关系，不做物理删除。
const NOTE_TAG_INDEX: OneToMany<BlockId, BlockId> = OneToMany::new("TagToNotes");
// 向量索引：存储 note 的 embedding，可重建
const NOTE_VECTOR_INDEX: VectorStore<Vec<f32>> = VectorStore::new("NoteVectors", 384);
// ==========================================
// Note：访问器 + 写操作
// ==========================================
impl NoteRef {
    pub(crate) fn new(id: Uuid, deleted: bool) -> Self {
        Self { id, deleted }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    pub fn hydrate(&self, reader: &NoteReader<'_>) -> Result<Option<Note>, redb::Error> {
        reader.get_by_id(&self.id)
    }

    pub fn edit(
        self,
        tx: &WriteTransaction,
        new_content: String,
        tags: Vec<Tag>,
    ) -> Result<Note, redb::Error> {
        let new_note = Note::create(tx, new_content, tags)?;
        NOTE_EDIT.link(tx, &self.id, &new_note.id)?;
        Ok(new_note)
    }

    pub fn reply_to(self, tx: &WriteTransaction, child_id: &Uuid) -> Result<(), redb::Error> {
        NOTE_LINK.link(tx, &self.id, child_id)
    }

    pub fn link_to_parent(
        self,
        tx: &WriteTransaction,
        parent_id: &Uuid,
    ) -> Result<(), redb::Error> {
        NOTE_LINK.link(tx, parent_id, &self.id)
    }

    pub fn del(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        NOTE_DELETE.add(tx, self.id.as_bytes()).map(|_| ())
    }

    pub fn restore(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        NOTE_DELETE.remove(tx, self.id.as_bytes()).map(|_| ())
    }
}

impl Note {
    /// 【系统级调用】强制初始化所有相关的数据表。
    /// 在 App 启动或建立新数据库时，必须且仅需调用一次！
    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        NOTE_STORE.init_table(tx)?;
        ID_ALIAS.init_table(tx)?;
        NOTE_EDIT.init_tables(tx)?;
        NOTE_DELETE.init_table(tx)?;
        NOTE_LINK.init_tables(tx)?;
        NOTE_TAG_INDEX.init_table(tx)?;
        NOTE_VECTOR_INDEX.init_table(tx)?;
        Ok(())
    }

    // ---------- 访问器 ----------

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn content(&self) -> &str {
        &self.inner.content
    }

    pub fn short_id(&self) -> &[u8; 8] {
        &self.inner.short_id
    }

    pub fn tags(&self) -> &[Uuid] {
        &self.inner.tags
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    pub fn note_ref(&self) -> NoteRef {
        NoteRef::new(self.id, self.deleted)
    }

    fn normalize_tag_ids(tags: Vec<Tag>) -> Vec<Uuid> {
        let tag_ids: Vec<Uuid> = tags.into_iter().map(|tag| tag.get_id()).collect();
        Self::dedup_tag_ids(tag_ids)
    }

    fn dedup_tag_ids(tag_ids: Vec<Uuid>) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        let mut normalized = Vec::with_capacity(tag_ids.len());

        for id in tag_ids {
            if seen.insert(id) {
                normalized.push(id);
            }
        }

        normalized
    }

    // ---------- 写操作 ----------

    /// 创建新笔记
    pub fn create(
        tx: &WriteTransaction,
        content: String,
        tags: Vec<Tag>,
    ) -> Result<Self, redb::Error> {
        let id = Uuid::now_v7();
        let short_id = random_id();
        let tag_ids = Self::normalize_tag_ids(tags);

        let block = NoteBlock {
            short_id,
            content,
            tags: tag_ids.clone(),
        };

        NOTE_STORE.put(tx, id.as_bytes(), &block)?;
        ID_ALIAS.put(tx, short_id, id.as_bytes())?;
        for tag_id in &tag_ids {
            let _ = NOTE_TAG_INDEX.add(tx, tag_id.as_bytes(), id.as_bytes())?;
        }

        Ok(Self {
            id,
            inner: block,
            deleted: false,
        })
    }

    /// 进化编辑：消耗旧版本，产出新版本
    pub fn edit(
        self,
        tx: &WriteTransaction,
        new_content: String,
        tags: Vec<Tag>,
    ) -> Result<Self, redb::Error> {
        self.note_ref().edit(tx, new_content, tags)
    }

    /// 建立父→子链接
    pub fn reply(&self, tx: &WriteTransaction, child: &Note) -> Result<(), redb::Error> {
        self.note_ref().reply_to(tx, &child.id)
    }

    /// 建立子→父链接（反向视角）
    pub fn link_to_parent(&self, tx: &WriteTransaction, parent: &Note) -> Result<(), redb::Error> {
        self.note_ref().link_to_parent(tx, &parent.id)
    }

    /// 删除
    pub fn del(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        self.note_ref().del(tx)
    }

    /// 取消删除标记
    pub fn restore(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        self.note_ref().restore(tx)
    }

    fn filter_search_text(content: &str) -> String {
        sanitize_search_text(content)
    }

    pub fn to_sync_record(&self) -> NoteSyncRecord {
        NoteSyncRecord {
            id: self.id,
            content: self.inner.content.clone(),
            short_id: self.inner.short_id,
            tags: self.inner.tags.clone(),
        }
    }

    pub fn import(tx: &WriteTransaction, record: NoteSyncRecord) -> Result<Self, redb::Error> {
        let note_id = record.id;
        let tag_ids = Self::dedup_tag_ids(record.tags);
        let block = NoteBlock {
            content: record.content,
            short_id: record.short_id,
            tags: tag_ids.clone(),
        };

        NOTE_STORE.put(tx, note_id.as_bytes(), &block)?;

        let mut alias_table = tx.open_table(ID_ALIAS.table_def())?;
        let note_id_bytes = note_id.into_bytes();
        let alias_value = postcard::to_allocvec(&note_id_bytes).map_err(|e| {
            redb::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        let should_insert_alias = match alias_table.get(block.short_id)? {
            Some(existing) => {
                let existing_id: BlockId = postcard::from_bytes(existing.value()).map_err(|e| {
                    redb::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                })?;
                existing_id == note_id_bytes
            }
            None => true,
        };
        if should_insert_alias {
            alias_table.insert(block.short_id, alias_value.as_slice())?;
        }

        for tag_id in &tag_ids {
            let _ = NOTE_TAG_INDEX.add(tx, tag_id.as_bytes(), note_id.as_bytes())?;
        }

        Ok(Self {
            id: note_id,
            inner: block,
            deleted: false,
        })
    }

    pub fn import_reply_link(
        tx: &WriteTransaction,
        parent_id: &Uuid,
        child_id: &Uuid,
    ) -> Result<(), redb::Error> {
        NOTE_LINK.link(tx, parent_id, child_id)
    }

    pub fn import_edit_link(
        tx: &WriteTransaction,
        previous_id: &Uuid,
        next_id: &Uuid,
    ) -> Result<(), redb::Error> {
        NOTE_EDIT.link(tx, previous_id, next_id)
    }

    pub fn import_tombstone(tx: &WriteTransaction, note_id: &Uuid) -> Result<(), redb::Error> {
        NOTE_DELETE.add(tx, note_id.as_bytes()).map(|_| ())
    }
}
impl Searchable for Note {
    type Id = Uuid;
    fn get_id(&self) -> Self::Id {
        self.get_id()
    }
    fn get_search_text(&self) -> String {
        Self::filter_search_text(self.content())
    }
}

// NoteReader：持有 transaction 引用，所有表一次性打开
// 生命周期安全：编译器确保 Reader 不会超出 Transaction 生命周期
pub struct NoteReader<'a> {
    tx: &'a ReadTransaction,
    note_table: KvReader<BlockId, NoteBlock>,
    alias_table: KvReader<[u8; 8], BlockId>,
    link_dag: DagReader,
    edit_dag: DagReader,
    del_set: SetReader<BlockId>,
    tag_index: OneToManyReader<BlockId, BlockId>,
}

impl<'a> NoteReader<'a> {
    /// 创建 Reader，一次性打开所有表
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            tx,
            note_table: NOTE_STORE.reader(tx)?,
            alias_table: ID_ALIAS.reader(tx)?,
            link_dag: NOTE_LINK.reader(tx)?,
            edit_dag: NOTE_EDIT.reader(tx)?,
            del_set: NOTE_DELETE.reader(tx)?,
            tag_index: NOTE_TAG_INDEX.reader(tx)?,
        })
    }

    /// 获取 transaction 引用（供 View 层使用）
    pub fn tx(&self) -> &'a ReadTransaction {
        self.tx
    }

    // ---------- 查询 ----------

    pub fn get_ref_by_id(&self, id: &Uuid) -> Result<Option<NoteRef>, redb::Error> {
        if !self.note_table.contains(id.as_bytes())? {
            return Ok(None);
        }

        Ok(Some(NoteRef::new(
            *id,
            self.del_set.contains(id.as_bytes())?,
        )))
    }

    pub fn get_by_id(&self, id: &Uuid) -> Result<Option<Note>, redb::Error> {
        let block = self.note_table.get(id.as_bytes())?;
        let deleted = self.del_set.contains(id.as_bytes())?;
        Ok(block.map(|b| Note {
            id: *id,
            inner: b,
            deleted,
        }))
    }

    pub fn get_ref_by_short_id(&self, short_id: &[u8; 8]) -> Result<Option<NoteRef>, redb::Error> {
        match self.alias_table.get(short_id)? {
            Some(uuid_bytes) => self.get_ref_by_id(&Uuid::from_bytes(uuid_bytes)),
            None => Ok(None),
        }
    }

    pub fn get_by_short_id(&self, short_id: &[u8; 8]) -> Result<Option<Note>, redb::Error> {
        match self.alias_table.get(short_id)? {
            Some(uuid_bytes) => self.get_by_id(&Uuid::from_bytes(uuid_bytes)),
            None => Ok(None),
        }
    }

    pub fn note_by_time(
        &self,
    ) -> Result<
        impl DoubleEndedIterator<Item = Result<Uuid, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let id_iter = self.note_table.keys()?;
        Ok(id_iter.map(|item| item.map(|key_guard| Uuid::from_bytes(key_guard.value()))))
    }

    pub fn is_deleted(&self, id: &Uuid) -> Result<bool, redb::Error> {
        self.del_set.contains(id.as_bytes())
    }

    /// 原始 append-only 索引，可能包含旧版本和墓碑 note。
    pub fn tagged_note_ids(
        &self,
        tag: &Tag,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::Error> {
        let iter = self.tag_index.get(tag.get_id().as_bytes())?;
        Ok(iter.map(|guard_res| guard_res.map(|guard| Uuid::from_bytes(guard.value()))))
    }

    /// 读时过滤后的视图：跳过墓碑 note，但保留仍存活的旧版本。
    pub fn notes_with_tag(
        &self,
        tag: &Tag,
    ) -> Result<impl Iterator<Item = Result<Note, NoteError>> + '_, redb::Error> {
        let iter = self.tagged_note_ids(tag)?;

        Ok(iter.filter_map(move |id_res| match id_res {
            Ok(id) => match self.get_by_id(&id) {
                Ok(Some(note)) if !note.is_deleted() => Some(Ok(note)),
                Ok(Some(_)) => None,
                Ok(None) => Some(Err(NoteError::IdNotFound { id })),
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(NoteError::Db(e.into()))),
        }))
    }

    /// 当前 tag 视图：跳过墓碑，同时过滤掉已被后继版本覆盖的旧 note。
    pub fn latest_notes_with_tag(
        &self,
        tag: &Tag,
    ) -> Result<impl Iterator<Item = Result<Note, NoteError>> + '_, redb::Error> {
        let iter = self.notes_with_tag(tag)?;

        Ok(iter.filter_map(move |note_res| match note_res {
            Ok(note) => match self.next_versions(&note) {
                Ok(mut next_versions) => match next_versions.next() {
                    Some(Ok(_)) => None,
                    Some(Err(e)) => Some(Err(NoteError::Db(e.into()))),
                    None => Some(Ok(note)),
                },
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(e)),
        }))
    }

    pub fn deleted_note_ids(
        &self,
    ) -> Result<
        impl DoubleEndedIterator<Item = Result<Uuid, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self.del_set.iter()?;
        Ok(iter.map(|item| item.map(|(guard, _)| Uuid::from_bytes(guard.value()))))
    }

    // ---------- 拓扑遍历 ----------

    pub fn children(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.link_dag.get_children(&note.id)
    }

    pub fn parents(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.link_dag.get_parents(&note.id)
    }

    pub fn parents_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.link_dag.get_parents(id)
    }

    pub fn children_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.link_dag.get_children(id)
    }

    pub fn next_versions(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.next_versions_raw(&note.id)
    }

    pub fn previous_versions(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.previous_versions_raw(&note.id)
    }

    pub fn all_versions(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.all_versions_raw(&note.id)
    }

    pub fn next_versions_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.edit_dag.get_children(id)
    }

    pub fn previous_versions_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.edit_dag.get_parents(id)
    }

    pub fn all_versions_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut related = Vec::new();

        queue.push_back(*id);
        visited.insert(*id);

        while let Some(current_id) = queue.pop_front() {
            for parent_res in self.edit_dag.get_parents(&current_id)? {
                let parent_id = parent_res?;
                if visited.insert(parent_id) {
                    related.push(parent_id);
                    queue.push_back(parent_id);
                }
            }

            for child_res in self.edit_dag.get_children(&current_id)? {
                let child_id = child_res?;
                if visited.insert(child_id) {
                    related.push(child_id);
                    queue.push_back(child_id);
                }
            }
        }

        Ok(related
            .into_iter()
            .map(Result::<Uuid, redb::StorageError>::Ok))
    }

    pub fn other_versions(
        &self,
        note: &Note,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        self.other_versions_raw(&note.id)
    }

    pub fn other_versions_raw(
        &self,
        id: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::StorageError>
    {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut related = Vec::new();

        queue.push_back(*id);
        visited.insert(*id);

        while let Some(current_id) = queue.pop_front() {
            for parent_res in self.edit_dag.get_parents(&current_id)? {
                let parent_id = parent_res?;
                if visited.insert(parent_id) {
                    related.push(parent_id);
                    queue.push_back(parent_id);
                }
            }

            for child_res in self.edit_dag.get_children(&current_id)? {
                let child_id = child_res?;
                if visited.insert(child_id) {
                    related.push(child_id);
                    queue.push_back(child_id);
                }
            }
        }

        Ok(related
            .into_iter()
            .map(Result::<Uuid, redb::StorageError>::Ok))
    }

    pub fn has_next_version(&self, id: &Uuid) -> Result<bool, redb::StorageError> {
        let mut next_versions = self.next_versions_raw(id)?;
        Ok(next_versions.next().transpose()?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tag::TagWriter;
    use redb::Database;
    use redb::ReadableDatabase;
    use tempfile::NamedTempFile;
    // 假设你有 Tag 的 mock 构建方法，这里用空 Vec 代替方便测试

    // ==========================================
    // 辅助测试函数
    // ==========================================
    fn create_temp_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::create(temp_file.path()).unwrap();

        // 创世操作：开启全局第一个写事务，把所有表建好
        let write_txn = db.begin_write().unwrap();
        Note::init_schema(&write_txn).expect("Failed to initialize database schema");
        TagWriter::init_schema(&write_txn).expect("Failed to initialize tag schema");
        write_txn.commit().unwrap();

        db
    }

    // ==========================================
    // 测试 1：基础的创建与双向读取
    // ==========================================
    #[test]
    fn test_note_create_and_read() {
        let db = create_temp_db();

        // --- 写入流 ---
        let write_txn = db.begin_write().unwrap();
        let note = Note::create(&write_txn, "Hello Synap!".to_string(), vec![]).unwrap();

        // 记录 ID，测试完毕后释放写入锁
        let note_id = note.get_id();
        let short_id = *note.short_id();
        write_txn.commit().unwrap();

        // --- 读取流 ---
        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        // 1. 测试主键查找
        let found_by_id = reader.get_by_id(&note_id).unwrap().unwrap();
        assert_eq!(found_by_id.content(), "Hello Synap!");

        // 2. 测试短别名查找 (NanoID -> Uuid -> NoteBlock)
        let found_by_alias = reader.get_by_short_id(&short_id).unwrap().unwrap();
        assert_eq!(found_by_alias.get_id(), note_id);
        assert_eq!(found_by_alias.content(), "Hello Synap!");
    }

    // ==========================================
    // 测试 2：不可变版本的沿革 (Edit 时光机)
    // ==========================================
    #[test]
    fn test_note_edit_lineage() {
        let db = create_temp_db();

        // --- 写入流 ---
        let write_txn = db.begin_write().unwrap();
        let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![]).unwrap();
        let v1_id = v1.get_id();

        // 核心：消耗 v1 的所有权，产出 v2
        let v2 = v1
            .edit(&write_txn, "Version 2".to_string(), vec![])
            .unwrap();
        let v2_id = v2.get_id();
        write_txn.commit().unwrap();

        // --- 读取流 ---
        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let read_v1 = reader.get_by_id(&v1_id).unwrap().unwrap();
        let read_v2 = reader.get_by_id(&v2_id).unwrap().unwrap();

        // 验证两个独立版本都物理存在于数据库中
        assert_eq!(read_v1.content(), "Version 1");
        assert_eq!(read_v2.content(), "Version 2");

        // 验证正向时光机：v1 的下一个版本是谁？
        let mut next_iters = reader.next_versions(&read_v1).unwrap();
        assert_eq!(next_iters.next().unwrap().unwrap(), v2_id);
        assert!(next_iters.next().is_none());

        // 验证逆向时光机：v2 是从谁变过来的？
        let mut prev_iters = reader.previous_versions(&read_v2).unwrap();
        assert_eq!(prev_iters.next().unwrap().unwrap(), v1_id);
    }

    #[test]
    fn test_note_other_versions_walks_entire_edit_component() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![]).unwrap();
        let v1_id = v1.get_id();

        let v2a = v1
            .clone()
            .edit(&write_txn, "Version 2A".to_string(), vec![])
            .unwrap();
        let v2a_id = v2a.get_id();

        let v2b = v1
            .edit(&write_txn, "Version 2B".to_string(), vec![])
            .unwrap();
        let v2b_id = v2b.get_id();

        let v3 = v2a
            .clone()
            .edit(&write_txn, "Version 3".to_string(), vec![])
            .unwrap();
        let v3_id = v3.get_id();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();
        let read_v2a = reader.get_by_id(&v2a_id).unwrap().unwrap();

        let others: Vec<Uuid> = reader
            .other_versions(&read_v2a)
            .unwrap()
            .map(|res| res.unwrap())
            .collect();

        assert_eq!(others.len(), 3);
        assert!(others.contains(&v1_id));
        assert!(others.contains(&v2b_id));
        assert!(others.contains(&v3_id));
        assert!(!others.contains(&v2a_id));
    }

    // ==========================================
    // 测试 3：图谱连线的拓扑派生 (Reply & Link)
    // ==========================================
    #[test]
    fn test_note_topology_links() {
        let db = create_temp_db();

        // --- 写入流 ---
        let write_txn = db.begin_write().unwrap();

        // 创建三个游离节点
        let parent = Note::create(&write_txn, "Root Idea".to_string(), vec![]).unwrap();
        let child_1 = Note::create(&write_txn, "Sub Idea 1".to_string(), vec![]).unwrap();
        let child_2 = Note::create(&write_txn, "Sub Idea 2".to_string(), vec![]).unwrap();

        let parent_id = parent.get_id();
        let c1_id = child_1.get_id();
        let c2_id = child_2.get_id();

        // 测试双向链接方法（等价操作）
        parent.reply(&write_txn, &child_1).unwrap(); // parent 主动连接 child_1
        child_2.link_to_parent(&write_txn, &parent).unwrap(); // child_2 主动认父 parent

        write_txn.commit().unwrap();

        // --- 读取流 ---
        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let read_parent = reader.get_by_id(&parent_id).unwrap().unwrap();
        let read_c1 = reader.get_by_id(&c1_id).unwrap().unwrap();

        // 1. 验证向下推演（孩子有谁？）
        let children: Vec<Uuid> = reader
            .children(&read_parent)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(children.len(), 2);
        assert!(children.contains(&c1_id));
        assert!(children.contains(&c2_id));

        // 2. 验证向上溯源（父亲是谁？）
        let c1_parents: Vec<Uuid> = reader
            .parents(&read_c1)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(c1_parents.len(), 1);
        assert_eq!(c1_parents[0], parent_id);
    }

    #[test]
    fn test_note_tag_index_is_append_only_across_edits() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let tag_writer = TagWriter::new(&write_txn);
        let rust = tag_writer.find_or_create("rust").unwrap();
        let async_tag = tag_writer.find_or_create("async").unwrap();

        let v1 = Note::create(&write_txn, "learn rust".to_string(), vec![rust.clone()]).unwrap();
        let v1_id = v1.get_id();

        let v2 = v1
            .edit(
                &write_txn,
                "learn rust async".to_string(),
                vec![rust.clone(), async_tag.clone()],
            )
            .unwrap();
        let v2_id = v2.get_id();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let rust_ids: Vec<Uuid> = reader
            .tagged_note_ids(&rust)
            .unwrap()
            .map(|res| res.unwrap())
            .collect();
        assert_eq!(rust_ids.len(), 2);
        assert!(rust_ids.contains(&v1_id));
        assert!(rust_ids.contains(&v2_id));

        let async_ids: Vec<Uuid> = reader
            .tagged_note_ids(&async_tag)
            .unwrap()
            .map(|res| res.unwrap())
            .collect();
        assert_eq!(async_ids, vec![v2_id]);

        let live_rust_notes: Vec<Uuid> = reader
            .notes_with_tag(&rust)
            .unwrap()
            .map(|res| res.unwrap().get_id())
            .collect();
        assert_eq!(live_rust_notes.len(), 2);
        assert!(live_rust_notes.contains(&v1_id));
        assert!(live_rust_notes.contains(&v2_id));
    }

    #[test]
    fn test_note_tag_index_keeps_tombstoned_entries_but_filters_them_on_read() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let tag_writer = TagWriter::new(&write_txn);
        let rust = tag_writer.find_or_create("rust").unwrap();

        let note = Note::create(&write_txn, "ephemeral".to_string(), vec![rust.clone()]).unwrap();
        let note_id = note.get_id();
        note.del(&write_txn).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let indexed_ids: Vec<Uuid> = reader
            .tagged_note_ids(&rust)
            .unwrap()
            .map(|res| res.unwrap())
            .collect();
        assert_eq!(indexed_ids, vec![note_id]);

        let visible_notes: Vec<Uuid> = reader
            .notes_with_tag(&rust)
            .unwrap()
            .map(|res| res.unwrap().get_id())
            .collect();
        assert!(visible_notes.is_empty());
    }

    #[test]
    fn test_note_latest_notes_with_tag_filters_superseded_versions() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let tag_writer = TagWriter::new(&write_txn);
        let rust = tag_writer.find_or_create("rust").unwrap();
        let async_tag = tag_writer.find_or_create("async").unwrap();

        let v1 = Note::create(&write_txn, "learn rust".to_string(), vec![rust.clone()]).unwrap();
        let _v2 = v1
            .edit(&write_txn, "learn async".to_string(), vec![async_tag])
            .unwrap();
        let live = Note::create(&write_txn, "ship rust".to_string(), vec![rust.clone()]).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let visible_notes: Vec<Uuid> = reader
            .latest_notes_with_tag(&rust)
            .unwrap()
            .map(|res| res.unwrap().get_id())
            .collect();
        assert_eq!(visible_notes, vec![live.get_id()]);
    }

    #[test]
    fn test_note_create_deduplicates_repeated_tags() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let tag_writer = TagWriter::new(&write_txn);
        let rust = tag_writer.find_or_create("rust").unwrap();

        let note = Note::create(
            &write_txn,
            "dedupe me".to_string(),
            vec![rust.clone(), rust.clone()],
        )
        .unwrap();
        let note_id = note.get_id();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();
        let stored_note = reader.get_by_id(&note_id).unwrap().unwrap();

        assert_eq!(stored_note.tags().len(), 1);
        assert_eq!(stored_note.tags()[0], rust.get_id());

        let indexed_ids: Vec<Uuid> = reader
            .tagged_note_ids(&rust)
            .unwrap()
            .map(|res| res.unwrap())
            .collect();
        assert_eq!(indexed_ids, vec![note_id]);
    }

    #[test]
    fn test_note_restore_clears_tombstone() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let note = Note::create(&write_txn, "restore me".to_string(), vec![]).unwrap();
        let note_id = note.get_id();
        note.clone().del(&write_txn).unwrap();
        note.restore(&write_txn).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();
        let restored = reader.get_by_id(&note_id).unwrap().unwrap();
        assert!(!restored.is_deleted());

        let deleted_ids: Vec<Uuid> = reader
            .deleted_note_ids()
            .unwrap()
            .map(|res| res.unwrap())
            .collect();
        assert!(deleted_ids.is_empty());
    }

    #[test]
    fn test_note_search_text_filters_markdown_images_and_data_uris() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let note = Note::create(
            &write_txn,
            "hello ![cover](data:image/png;base64,AAAA) world data:image/jpeg;base64,BBBB"
                .to_string(),
            vec![],
        )
        .unwrap();
        write_txn.commit().unwrap();

        let search_text = note.get_search_text();
        assert_eq!(search_text, "hello world");
        assert!(!search_text.contains("data:image/"));
        assert!(!search_text.contains("!["));
    }
}
