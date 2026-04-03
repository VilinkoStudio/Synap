use crate::{
    db::{
        kvstore::{KvReader, KvStore},
        types::BlockId,
    },
    search::types::Searchable,
};
use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// 静态表定义
const TAG_STORE: KvStore<BlockId, TagBlock> = KvStore::new("TagBlocks");
const TAG_NAMESPACE: Uuid = Uuid::from_u128(0x76a173b2bb8d5e91a6f8fd1d0ec0b76f);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagBlock {
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagSyncRecord {
    pub id: Uuid,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    id: Uuid,
    inner: TagBlock,
}

// ==========================================
// Tag：纯数据结构
// ==========================================
impl Tag {
    pub fn normalize_content(content: &str) -> Option<String> {
        let normalized = content.trim();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.to_owned())
        }
    }

    pub fn id_for_content(content: &str) -> Option<Uuid> {
        Self::normalize_content(content)
            .map(|normalized| Self::id_for_normalized_content(&normalized))
    }

    fn id_for_normalized_content(content: &str) -> Uuid {
        Uuid::new_v5(&TAG_NAMESPACE, content.as_bytes())
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_content(&self) -> &str {
        &self.inner.content
    }

    pub fn to_sync_record(&self) -> TagSyncRecord {
        TagSyncRecord {
            id: self.id,
            content: self.inner.content.clone(),
        }
    }
}

impl Searchable for Tag {
    type Id = Uuid;
    fn get_id(&self) -> Self::Id {
        self.get_id()
    }
    fn get_search_text(&self) -> String {
        self.inner.content.clone()
    }
}

// ==========================================
// TagReader：数据库视图层
// ==========================================
pub struct TagReader<'a> {
    tag_table: KvReader<BlockId, TagBlock>,
    _marker: std::marker::PhantomData<&'a ReadTransaction>,
}

impl<'a> TagReader<'a> {
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            tag_table: TAG_STORE.reader(tx)?,
            _marker: std::marker::PhantomData,
        })
    }

    // ---------- 查询 ----------

    pub fn get_by_id(&self, id: &Uuid) -> Result<Option<Tag>, redb::Error> {
        let block = self.tag_table.get(id.as_bytes())?;
        Ok(block.map(|inner| Tag { id: *id, inner }))
    }

    pub fn find_by_content(&self, content: &str) -> Result<Option<Tag>, redb::Error> {
        let Some(id) = Tag::id_for_content(content) else {
            return Ok(None);
        };
        self.get_by_id(&id)
    }

    pub fn all(
        &self,
    ) -> Result<impl Iterator<Item = Result<Tag, redb::StorageError>> + '_, redb::StorageError>
    {
        let iter = self.tag_table.iter()?;
        Ok(iter.map(|item| {
            item.map(|(key_guard, inner)| Tag {
                id: Uuid::from_bytes(key_guard.value()),
                inner,
            })
        }))
    }
}

// ==========================================
// TagWriter：写操作层
// ==========================================
pub struct TagWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> TagWriter<'a> {
    pub fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    /// 初始化 tag 相关表。
    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        TAG_STORE.init_table(tx)?;
        Ok(())
    }

    /// Tag 是不可变块：只创建新实体，或读取已存在的同内容实体。
    pub fn find_or_create(&self, content: impl AsRef<str>) -> Result<Tag, redb::Error> {
        let normalized = Tag::normalize_content(content.as_ref()).ok_or_else(|| {
            redb::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tag content cannot be empty",
            ))
        })?;
        let id = Tag::id_for_normalized_content(&normalized);

        let existing_inner = TAG_STORE.get_in_write(self.tx, id.as_bytes())?;

        if let Some(inner) = existing_inner {
            return Ok(Tag { id, inner });
        }

        let block = TagBlock {
            content: normalized,
        };

        TAG_STORE.put(self.tx, id.as_bytes(), &block)?;

        Ok(Tag { id, inner: block })
    }

    /// 导入远端 tag 记录。
    /// 新逻辑下 tag ID 必须是 UUID v5(content)，因此同内容 tag 会自然收敛到同一个实体。
    pub fn import(&self, record: TagSyncRecord) -> Result<Tag, redb::Error> {
        let normalized = Tag::normalize_content(&record.content).ok_or_else(|| {
            redb::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tag content cannot be empty",
            ))
        })?;
        let expected_id = Tag::id_for_normalized_content(&normalized);
        if record.id != expected_id {
            return Err(redb::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tag id does not match UUID v5(content)",
            )));
        }

        let existing_inner = TAG_STORE.get_in_write(self.tx, expected_id.as_bytes())?;

        if let Some(inner) = existing_inner {
            if inner.content != normalized {
                return Err(redb::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "tag id/content conflict during sync import",
                )));
            }

            return Ok(Tag {
                id: record.id,
                inner,
            });
        }

        let block = TagBlock {
            content: normalized,
        };
        TAG_STORE.put(self.tx, expected_id.as_bytes(), &block)?;

        Ok(Tag {
            id: expected_id,
            inner: block,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    fn create_temp_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::create(temp_file.path()).unwrap();

        let write_txn = db.begin_write().unwrap();
        TagWriter::init_schema(&write_txn).unwrap();
        write_txn.commit().unwrap();

        db
    }

    #[test]
    fn test_tag_create_and_read() {
        let db = create_temp_db();

        // 写入
        let write_txn = db.begin_write().unwrap();
        let writer = TagWriter::new(&write_txn);
        let tag = writer.find_or_create("rust").unwrap();
        let tag_id = tag.get_id();
        write_txn.commit().unwrap();

        // 读取
        let read_txn = db.begin_read().unwrap();
        let reader = TagReader::new(&read_txn).unwrap();
        let found = reader.get_by_id(&tag_id).unwrap().unwrap();
        assert_eq!(found.get_content(), "rust");
    }

    #[test]
    fn test_find_or_create() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let writer = TagWriter::new(&write_txn);

        // 第一次创建
        let tag1 = writer.find_or_create("rust").unwrap();
        let id1 = tag1.get_id();

        // 第二次查找（应该返回相同的 tag）
        let tag2 = writer.find_or_create("rust").unwrap();
        assert_eq!(tag2.get_id(), id1);
        assert_eq!(tag2.get_content(), "rust");
        assert_eq!(Some(id1), Tag::id_for_content("rust"));
        assert_eq!(Some(id1), Tag::id_for_content(" rust "));

        write_txn.commit().unwrap();
    }

    #[test]
    fn test_find_by_content_without_secondary_index() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let writer = TagWriter::new(&write_txn);
        let created = writer.find_or_create("rust").unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = TagReader::new(&read_txn).unwrap();
        let found = reader.find_by_content(" rust ").unwrap().unwrap();

        assert_eq!(found.get_id(), created.get_id());
        assert_eq!(found.get_content(), "rust");
    }

    #[test]
    fn test_import_rejects_mismatched_uuid_v5() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let writer = TagWriter::new(&write_txn);
        let err = writer
            .import(TagSyncRecord {
                id: Uuid::now_v7(),
                content: "rust".to_string(),
            })
            .unwrap_err();

        assert!(matches!(err, redb::Error::Io(_)));
    }
}
