use redb::{
    ReadOnlyTable, ReadTransaction, ReadableTable, ReadableTableMetadata, TableDefinition,
    WriteTransaction,
};

/// 静态表定义：SetStore - 仅存储 Key 的集合，无 Value
/// 底层使用 TableDefinition<K, ()> 实现，空元组作为占位 Value
pub struct SetStore<K: redb::Key + 'static> {
    def: TableDefinition<'static, K, ()>,
}

impl<K: redb::Key + 'static> SetStore<K> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            def: TableDefinition::new(name),
        }
    }

    /// 强制在物理磁盘上物化（创建）这张表
    pub fn init_table(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        let _ = tx.open_table(self.def)?;
        Ok(())
    }

    /// 添加元素到集合
    /// 返回 true 表示元素已存在，false 表示新插入
    pub fn add<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let existed = table.insert(key, ())?.is_some();
        Ok(existed)
    }

    /// 从集合中移除元素
    /// 返回 true 表示元素存在并被删除，false 表示元素不存在
    pub fn remove<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        let mut table = tx.open_table(self.def)?;
        let removed = table.remove(key)?;
        Ok(removed.is_some())
    }

    /// 检查元素是否在集合中
    pub fn contains<'k>(
        &self,
        tx: &ReadTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        let table = tx.open_table(self.def)?;
        let exists = table.get(key)?.is_some();
        Ok(exists)
    }

    pub fn contains_in_write<'k>(
        &self,
        tx: &WriteTransaction,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        let table = tx.open_table(self.def)?;
        let contains = table.get(key)?.is_some();
        Ok(contains)
    }

    /// 获取 Reader，用于读取操作
    pub fn reader(&self, tx: &ReadTransaction) -> Result<SetReader<K>, redb::Error> {
        let table = tx.open_table(self.def)?;
        Ok(SetReader {
            table,
            _marker: std::marker::PhantomData,
        })
    }
}

/// 专用的 Reader 结构体，持有 Table，提供安全的惰性迭代
pub struct SetReader<K: redb::Key + 'static> {
    table: ReadOnlyTable<K, ()>,
    _marker: std::marker::PhantomData<K>,
}

impl<K: redb::Key + 'static> SetReader<K> {
    /// 检查元素是否在集合中
    pub fn contains<'k>(
        &self,
        key: impl std::borrow::Borrow<K::SelfType<'k>>,
    ) -> Result<bool, redb::Error> {
        Ok(self.table.get(key)?.is_some())
    }

    /// 按字典序遍历所有 Key
    /// 返回 redb::Range 迭代器，元素为 Result<(K::SelfType, ()), redb::StorageError>
    /// 调用者可通过 .map(|(k, _)| k.value()) 提取 Key
    pub fn iter<'a>(&'a self) -> Result<redb::Range<'a, K, ()>, redb::StorageError>
    where
        K: std::borrow::Borrow<K::SelfType<'a>>,
    {
        self.table.range::<K>(..)
    }

    /// 按字典序遍历指定范围
    pub fn range<'a, R>(&'a self, range: R) -> Result<redb::Range<'a, K, ()>, redb::StorageError>
    where
        K: std::borrow::Borrow<K::SelfType<'a>>,
        R: std::ops::RangeBounds<K>,
    {
        self.table.range(range)
    }

    /// 获取集合中元素的数量
    pub fn len(&self) -> Result<u64, redb::StorageError> {
        self.table.len()
    }

    /// 检查集合是否为空
    pub fn is_empty(&self) -> Result<bool, redb::StorageError> {
        Ok(self.len()? == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::BlockId;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

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
    fn test_set_add_remove_contains() {
        let db = temp_db();
        let set: SetStore<BlockId> = SetStore::new("test_set");

        // 初始化表
        let wtx = db.begin_write().unwrap();
        set.init_table(&wtx).unwrap();
        wtx.commit().unwrap();

        // 添加元素
        let wtx = db.begin_write().unwrap();
        assert!(!set.add(&wtx, &id(1)).unwrap()); // 新插入，返回 false
        assert!(set.add(&wtx, &id(1)).unwrap()); // 已存在，返回 true
        assert!(!set.add(&wtx, &id(2)).unwrap());
        assert!(!set.add(&wtx, &id(3)).unwrap());
        wtx.commit().unwrap();

        // 检查包含关系
        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        assert!(reader.contains(&id(1)).unwrap());
        assert!(reader.contains(&id(2)).unwrap());
        assert!(!reader.contains(&id(99)).unwrap());

        // 删除元素
        let wtx = db.begin_write().unwrap();
        assert!(set.remove(&wtx, &id(1)).unwrap()); // 存在，删除成功
        assert!(!set.remove(&wtx, &id(99)).unwrap()); // 不存在
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        assert!(!reader.contains(&id(1)).unwrap());
        assert!(reader.contains(&id(2)).unwrap());
    }

    #[test]
    fn test_set_iter_dict_order() {
        let db = temp_db();
        let set: SetStore<BlockId> = SetStore::new("test_set_iter");

        // 初始化表
        let wtx = db.begin_write().unwrap();
        set.init_table(&wtx).unwrap();
        wtx.commit().unwrap();

        // 乱序插入
        let wtx = db.begin_write().unwrap();
        for n in [3u8, 1, 5, 2, 4] {
            set.add(&wtx, &id(n)).unwrap();
        }
        wtx.commit().unwrap();

        // 按字典序遍历
        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        let results: Vec<_> = reader
            .iter()
            .unwrap()
            .map(|res| {
                let (k_guard, _) = res.unwrap();
                k_guard.value() // 返回 &[u8; 16]
            })
            .collect();

        // BlockId 是 [u8; 16]，最后一个字节是我们设置的值
        assert_eq!(results.len(), 5);
        assert_eq!(results[0][15], 1);
        assert_eq!(results[1][15], 2);
        assert_eq!(results[2][15], 3);
        assert_eq!(results[3][15], 4);
        assert_eq!(results[4][15], 5);
    }

    #[test]
    fn test_set_range_query() {
        let db = temp_db();
        let set: SetStore<BlockId> = SetStore::new("test_set_range");

        // 初始化表
        let wtx = db.begin_write().unwrap();
        set.init_table(&wtx).unwrap();
        wtx.commit().unwrap();

        // 插入 1-10
        let wtx = db.begin_write().unwrap();
        for n in 1u8..=10 {
            set.add(&wtx, &id(n)).unwrap();
        }
        wtx.commit().unwrap();

        // 范围查询 [3, 7)
        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        let results: Vec<_> = reader
            .range(&id(3)..&id(7))
            .unwrap()
            .map(|res| {
                let (k_guard, _) = res.unwrap();
                k_guard.value()[15]
            })
            .collect();

        assert_eq!(results, vec![3, 4, 5, 6]);
    }

    #[test]
    fn test_set_collect_keys() {
        let db = temp_db();
        let set: SetStore<BlockId> = SetStore::new("test_set_collect");

        // 初始化表
        let wtx = db.begin_write().unwrap();
        set.init_table(&wtx).unwrap();
        wtx.commit().unwrap();

        // 插入元素
        let wtx = db.begin_write().unwrap();
        set.add(&wtx, &id(10)).unwrap();
        set.add(&wtx, &id(20)).unwrap();
        set.add(&wtx, &id(30)).unwrap();
        wtx.commit().unwrap();

        // 收集所有 key 到 Vec
        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        let keys: Vec<BlockId> = reader
            .iter()
            .unwrap()
            .map(|res| {
                let (k_guard, _) = res.unwrap();
                k_guard.value() // BlockId ([u8; 16]) 实现了 Copy，直接返回值
            })
            .collect();

        assert_eq!(keys.len(), 3);
        // BlockId 按字节字典序排列
        assert!(keys.iter().any(|k| k[15] == 10));
        assert!(keys.iter().any(|k| k[15] == 20));
        assert!(keys.iter().any(|k| k[15] == 30));
    }

    #[test]
    fn test_set_len_and_is_empty() {
        let db = temp_db();
        let set: SetStore<BlockId> = SetStore::new("test_set_len");

        // 初始化表
        let wtx = db.begin_write().unwrap();
        set.init_table(&wtx).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        assert!(reader.is_empty().unwrap());
        assert_eq!(reader.len().unwrap(), 0);

        // 添加元素
        let wtx = db.begin_write().unwrap();
        set.add(&wtx, &id(1)).unwrap();
        set.add(&wtx, &id(2)).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = set.reader(&rtx).unwrap();
        assert!(!reader.is_empty().unwrap());
        assert_eq!(reader.len().unwrap(), 2);
    }

    #[test]
    fn test_set_reader_outlives_definition() {
        let db = temp_db();

        // 写入测试数据
        let wtx = db.begin_write().unwrap();
        let set: SetStore<BlockId> = SetStore::new("outlive_set");
        set.init_table(&wtx).unwrap();
        set.add(&wtx, &id(7)).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = {
            let local: SetStore<BlockId> = SetStore::new("outlive_set");
            local.reader(&rtx).unwrap()
        }; // local 已销毁

        // reader 依然可用
        assert!(reader.contains(&id(7)).unwrap());
        let count = reader.iter().unwrap().count();
        assert_eq!(count, 1);
    }
}
