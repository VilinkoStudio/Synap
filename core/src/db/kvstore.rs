use redb::{ReadOnlyTable, ReadTransaction, TableDefinition, WriteTransaction};
use serde::{de::DeserializeOwned, Serialize};

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
        let bytes = postcard::to_allocvec(value).expect("serialize failed");
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

    /// 强制在物理磁盘上物化（创建）这张表
    pub fn init_table(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        let _ = tx.open_table(self.def)?;
        Ok(())
    }
}

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
            Some(guard) => {
                let v = postcard::from_bytes(guard.value()).expect("deserialize failed");
                Ok(Some(v))
            }
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
                let value = postcard::from_bytes(v_guard.value()).expect("deserialize failed");
                (k_guard, value)
            })
        })
    }
}

impl<'a, K: redb::Key + 'static, V: DeserializeOwned> DoubleEndedIterator for KvIter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|res| {
            res.map(|(k_guard, v_guard)| {
                let value = postcard::from_bytes(v_guard.value()).expect("deserialize failed");
                (k_guard, value)
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::BlockId;
    use redb::{Database, ReadableDatabase};
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
}
