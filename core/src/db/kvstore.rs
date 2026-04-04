use std::io;

use redb::{ReadOnlyTable, ReadTransaction, ReadableTable, TableDefinition, WriteTransaction};
use serde::{de::DeserializeOwned, Serialize};

use crate::db::codec;

/// 类型化 KV 的静态蓝图。
///
/// `KvStore` 本身不持有 transaction，也不持有打开后的 table；
/// 它只是“表名 + 类型信息 + value codec 入口”。
///
/// 这让它很适合做模块级 `const` 定义，而真正和事务生命周期绑定的读取能力
/// 则放在 [`KvReader`] 上。
///
/// 约定上：
///
/// - `KvStore` 主要承载写操作和少量 write-tx helper
/// - `KvReader` 主要承载正式的读操作和惰性迭代
pub struct KvStore<K: redb::Key + 'static, V> {
    pub(crate) def: TableDefinition<'static, K, &'static [u8]>,
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K: redb::Key + 'static, V: Serialize + DeserializeOwned> KvStore<K, V> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            def: TableDefinition::new(name),
            _marker: std::marker::PhantomData,
        }
    }

    /// 获取表定义（供需要直接操作表的场景使用）
    pub const fn table_def(&self) -> TableDefinition<'static, K, &'static [u8]> {
        self.def
    }

    pub fn put<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
        value: &V,
    ) -> Result<(), redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let bytes = codec::encode_default(value).map_err(codec_to_redb_error)?;
        table.insert(key, bytes.as_slice())?;
        Ok(())
    }

    pub fn delete<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let removed = table.remove(key)?;
        Ok(removed.is_some())
    }

    pub fn reader(&self, tx: &ReadTransaction) -> Result<KvReader<K, V>, redb::Error> {
        let table = tx.open_table(self.def)?;
        Ok(KvReader {
            table,
            _marker: std::marker::PhantomData,
        })
    }

    /// 在写事务中按 `KvStore` 的 codec 规则读取 typed value。
    ///
    /// 这个方法的用途是“读后写”场景，例如幂等插入、冲突检查、merge 等。
    /// 它避免了模型层绕过 codec，直接拿原始 bytes 做 `postcard` 解析。
    pub fn get_in_write<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<Option<V>, redb::Error> {
        let table = tx.open_table(self.def)?;
        let value = table.get(key)?;
        match value {
            Some(guard) => Ok(Some(decode_value(guard.value())?)),
            None => Ok(None),
        }
    }

    /// 强制在物理磁盘上物化（创建）这张表
    pub fn init_table(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        let _ = tx.open_table(self.def)?;
        Ok(())
    }
}

/// 绑定到某个 `ReadTransaction` 的读视图。
///
/// `KvReader` 持有真正打开过的 `ReadOnlyTable`，因此它是惰性 iterator
/// 的稳定 owner。所有借用 table 的读取游标都应该从这里往外借出，而不是
/// 在临时打开 table 的函数里直接返回。
pub struct KvReader<K: redb::Key + 'static, V> {
    table: ReadOnlyTable<K, &'static [u8]>,
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K: redb::Key + 'static, V: DeserializeOwned> KvReader<K, V> {
    pub fn get<'a, 'k>(
        &'a self,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<Option<V>, redb::Error> {
        match self.table.get(key)? {
            Some(guard) => Ok(Some(decode_value(guard.value())?)),
            None => Ok(None),
        }
    }

    pub fn contains<'k>(
        &self,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        Ok(self.table.get(key)?.is_some())
    }

    /// 返回迭代器，自动反序列化值为 V
    pub fn iter<'a>(&'a self) -> Result<KvIter<'a, K, V>, redb::StorageError>
    where
        K: std::borrow::Borrow<K::SelfType<'a>>,
    {
        let range = self.table.range::<K>(..)?;
        Ok(KvIter {
            inner: range,
            _marker: std::marker::PhantomData,
        })
    }

    /// 只遍历 key，不反序列化 value
    pub fn keys<'a>(&'a self) -> Result<KvKeyIter<'a, K>, redb::StorageError>
    where
        K: std::borrow::Borrow<K::SelfType<'a>>,
    {
        let range = self.table.range::<K>(..)?;
        Ok(KvKeyIter { inner: range })
    }

    /// 只遍历指定范围内的 key，不反序列化 value
    pub fn keys_range<'a, R>(&'a self, range: R) -> Result<KvKeyIter<'a, K>, redb::StorageError>
    where
        K: std::borrow::Borrow<K::SelfType<'a>>,
        R: std::ops::RangeBounds<K>,
    {
        let range = self.table.range(range)?;
        Ok(KvKeyIter { inner: range })
    }
}

/// KvStore 的迭代器，自动反序列化值
pub struct KvIter<'a, K: redb::Key + 'static, V> {
    inner: redb::Range<'a, K, &'static [u8]>,
    _marker: std::marker::PhantomData<V>,
}

impl<'a, K: redb::Key + 'static, V: DeserializeOwned> Iterator for KvIter<'a, K, V> {
    type Item = Result<(redb::AccessGuard<'a, K>, V), redb::StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map(|(k_guard, v_guard)| {
                decode_iter_value(v_guard.value()).map(|value| (k_guard, value))
            })
            .and_then(|result| result.map_err(codec_to_storage_error))
        })
    }
}

impl<'a, K: redb::Key + 'static, V: DeserializeOwned> DoubleEndedIterator for KvIter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|res| {
            res.map(|(k_guard, v_guard)| {
                decode_iter_value(v_guard.value()).map(|value| (k_guard, value))
            })
            .and_then(|result| result.map_err(codec_to_storage_error))
        })
    }
}

/// 仅遍历 key 的迭代器
pub struct KvKeyIter<'a, K: redb::Key + 'static> {
    inner: redb::Range<'a, K, &'static [u8]>,
}

impl<'a, K: redb::Key + 'static> Iterator for KvKeyIter<'a, K> {
    type Item = Result<redb::AccessGuard<'a, K>, redb::StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| res.map(|(k_guard, _)| k_guard))
    }
}

impl<'a, K: redb::Key + 'static> DoubleEndedIterator for KvKeyIter<'a, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|res| res.map(|(k_guard, _)| k_guard))
    }
}

fn decode_value<V: DeserializeOwned>(bytes: &[u8]) -> Result<V, redb::Error> {
    codec::decode_default(bytes).map_err(codec_to_redb_error)
}

fn decode_iter_value<V: DeserializeOwned>(bytes: &[u8]) -> Result<V, codec::ValueCodecError> {
    codec::decode_default(bytes)
}

fn codec_to_redb_error(err: codec::ValueCodecError) -> redb::Error {
    redb::Error::from(io::Error::from(err))
}

fn codec_to_storage_error(err: codec::ValueCodecError) -> redb::StorageError {
    redb::StorageError::from(io::Error::from(err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::codec::ENVELOPE_MAGIC;
    use crate::db::types::BlockId;
    use redb::{Database, ReadableDatabase, ReadableTable};
    use serde::{Deserialize, Serialize};
    use tempfile::NamedTempFile;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct Block {
        content: String,
        version: u32,
    }

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    fn id(n: u8) -> BlockId {
        let mut b = [0u8; 16];
        b[15] = n;
        b
    }

    #[test]
    fn test_put_get_delete() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("blocks");

        let block = Block {
            content: "hello".into(),
            version: 1,
        };

        // put
        let wtx = db.begin_write().unwrap();
        store.put(&wtx, &id(1), &block).unwrap();
        let raw = {
            let table = wtx.open_table(store.table_def()).unwrap();
            let raw = table.get(&id(1)).unwrap().unwrap().value().to_vec();
            raw
        };
        assert!(raw.starts_with(&ENVELOPE_MAGIC));
        wtx.commit().unwrap();

        // get
        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        let got = reader.get(&id(1)).unwrap().unwrap();
        assert_eq!(got, block);
        assert!(reader.get(&id(99)).unwrap().is_none());

        // update (put 覆盖)
        let updated = Block {
            content: "world".into(),
            version: 2,
        };
        let wtx = db.begin_write().unwrap();
        store.put(&wtx, &id(1), &updated).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        assert_eq!(reader.get(&id(1)).unwrap().unwrap(), updated);

        // delete
        let wtx = db.begin_write().unwrap();
        assert!(store.delete(&wtx, &id(1)).unwrap());
        assert!(!store.delete(&wtx, &id(99)).unwrap());
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        assert!(reader.get(&id(1)).unwrap().is_none());
    }

    #[test]
    fn test_iter_dict_order() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("blocks_iter");

        // 故意乱序插入
        let wtx = db.begin_write().unwrap();
        for n in [3u8, 1, 2] {
            store
                .put(
                    &wtx,
                    &id(n),
                    &Block {
                        content: format!("block_{}", n),
                        version: n as u32,
                    },
                )
                .unwrap();
        }
        wtx.commit().unwrap();

        // 惰性迭代器，按字典序输出
        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        let results: Vec<_> = reader
            .iter()
            .unwrap()
            .map(|res| {
                let (k_guard, v) = res.unwrap();
                (k_guard.value()[15], v.content)
            })
            .collect();

        assert_eq!(
            results,
            vec![
                (1, "block_1".into()),
                (2, "block_2".into()),
                (3, "block_3".into()),
            ]
        );
    }

    #[test]
    fn test_reader_outlives_definition() {
        let db = temp_db();

        let wtx = db.begin_write().unwrap();
        let store: KvStore<BlockId, Block> = KvStore::new("outlive");
        store
            .put(
                &wtx,
                &id(7),
                &Block {
                    content: "ghost".into(),
                    version: 0,
                },
            )
            .unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = {
            let local: KvStore<BlockId, Block> = KvStore::new("outlive");
            local.reader(&rtx).unwrap()
        }; // local 已销毁

        // reader 依然可用
        let val = reader.get(&id(7)).unwrap().unwrap();
        assert_eq!(val.content, "ghost");
    }

    #[test]
    fn test_reader_compat_with_legacy_postcard_payload() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("legacy");
        let legacy = postcard::to_allocvec(&Block {
            content: "legacy".into(),
            version: 9,
        })
        .unwrap();

        let wtx = db.begin_write().unwrap();
        {
            let mut table = wtx.open_table(store.table_def()).unwrap();
            table.insert(&id(3), legacy.as_slice()).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        let value = reader.get(&id(3)).unwrap().unwrap();
        assert_eq!(value.content, "legacy");
        assert_eq!(value.version, 9);
    }

    #[test]
    fn test_put_uses_compression_for_large_payloads() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("compressed");
        let value = Block {
            content: "x".repeat(4096),
            version: 1,
        };
        let legacy = postcard::to_allocvec(&value).unwrap();

        let wtx = db.begin_write().unwrap();
        store.put(&wtx, &id(5), &value).unwrap();
        let raw = {
            let table = wtx.open_table(store.table_def()).unwrap();
            let raw = table.get(&id(5)).unwrap().unwrap().value().to_vec();
            raw
        };
        assert!(raw.starts_with(&ENVELOPE_MAGIC));
        assert!(raw.len() < legacy.len());
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        assert_eq!(reader.get(&id(5)).unwrap().unwrap(), value);
    }

    #[test]
    fn test_invalid_envelope_does_not_fallback_to_legacy() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("invalid_envelope");

        let mut invalid = Vec::from(ENVELOPE_MAGIC);
        invalid.extend_from_slice(&[1, 0, 0, 0]);
        invalid.extend_from_slice(&1u64.to_le_bytes());
        invalid.extend_from_slice(&1u64.to_le_bytes());

        let wtx = db.begin_write().unwrap();
        {
            let mut table = wtx.open_table(store.table_def()).unwrap();
            table.insert(&id(8), invalid.as_slice()).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();
        let err = reader.get(&id(8)).unwrap_err();
        assert!(matches!(
            err,
            redb::Error::Io(ref io_err) if io_err.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[test]
    fn test_contains_does_not_decode_invalid_value() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("contains_invalid_value");
        let invalid = b"not a valid postcard payload";

        let wtx = db.begin_write().unwrap();
        {
            let mut table = wtx.open_table(store.table_def()).unwrap();
            table.insert(&id(12), invalid.as_slice()).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();

        assert!(reader.contains(&id(12)).unwrap());
        assert!(matches!(
            reader.get(&id(12)),
            Err(redb::Error::Io(ref io_err)) if io_err.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[test]
    fn test_keys_do_not_decode_invalid_value() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("keys_invalid_value");
        let invalid = b"still not a valid postcard payload";

        let wtx = db.begin_write().unwrap();
        {
            let mut table = wtx.open_table(store.table_def()).unwrap();
            table.insert(&id(4), invalid.as_slice()).unwrap();
            table.insert(&id(9), invalid.as_slice()).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();

        let keys: Vec<u8> = reader
            .keys()
            .unwrap()
            .map(|item| item.unwrap().value()[15])
            .collect();

        assert_eq!(keys, vec![4, 9]);
    }

    #[test]
    fn test_keys_range_reads_only_requested_span() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("keys_range_span");

        let wtx = db.begin_write().unwrap();
        for n in 1u8..=5 {
            store
                .put(
                    &wtx,
                    &id(n),
                    &Block {
                        content: format!("block_{n}"),
                        version: n as u32,
                    },
                )
                .unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = store.reader(&rtx).unwrap();

        let keys: Vec<u8> = reader
            .keys_range(&id(2)..=&id(4))
            .unwrap()
            .map(|item| item.unwrap().value()[15])
            .collect();

        assert_eq!(keys, vec![2, 3, 4]);
    }

    #[test]
    fn test_get_in_write_uses_same_codec_path() {
        let db = temp_db();
        let store: KvStore<BlockId, Block> = KvStore::new("write_get");
        let block = Block {
            content: "hot".into(),
            version: 2,
        };

        let wtx = db.begin_write().unwrap();
        store.put(&wtx, &id(11), &block).unwrap();
        let value = store.get_in_write(&wtx, &id(11)).unwrap().unwrap();
        assert_eq!(value, block);
    }
}
