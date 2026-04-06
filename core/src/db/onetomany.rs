use redb::{MultimapTableDefinition, ReadOnlyMultimapTable, ReadTransaction, WriteTransaction};
use std::borrow::Borrow;

/// 静态表定义：仅仅是一个名称和类型的标识
pub struct OneToMany<K: redb::Key + 'static, V: redb::Key + 'static> {
    def: MultimapTableDefinition<'static, K, V>,
}

impl<K: redb::Key + 'static, V: redb::Key + 'static> OneToMany<K, V> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            def: MultimapTableDefinition::new(name),
        }
    }

    pub const fn table_def(&self) -> MultimapTableDefinition<'static, K, V> {
        self.def
    }

    // ==========================================
    // 写操作 (Create, Update, Delete)
    // 使用 impl Borrow<K::SelfType> 是 redb 官方推荐的泛型传参解法
    // ==========================================

    pub fn add<'k, 'v>(
        &self,
        tx: &WriteTransaction,
        k: impl Borrow<K::SelfType<'k>>,
        v: impl Borrow<V::SelfType<'v>>,
    ) -> Result<bool, redb::Error> {
        let mut table = tx.open_multimap_table(self.def)?;
        // insert 返回 bool，表示这对组合之前是否已经存在
        table.insert(k, v).map_err(|e| e.into())
    }

    pub fn remove<'k, 'v>(
        &self,
        tx: &WriteTransaction,
        k: impl Borrow<K::SelfType<'k>>,
        v: impl Borrow<V::SelfType<'v>>,
    ) -> Result<bool, redb::Error> {
        let mut table = tx.open_multimap_table(self.def)?;
        table.remove(k, v).map_err(|e| e.into())
    }

    pub fn remove_all<'k>(
        &self,
        tx: &WriteTransaction,
        k: impl Borrow<K::SelfType<'k>>,
    ) -> Result<(), redb::Error> {
        let mut table = tx.open_multimap_table(self.def)?;
        let _ = table.remove_all(k)?;
        Ok(())
    }

    pub fn reader(&self, tx: &ReadTransaction) -> Result<OneToManyReader<K, V>, redb::Error> {
        let table = tx.open_multimap_table(self.def)?;
        Ok(OneToManyReader { table })
    }

    /// 强制在物理磁盘上物化（创建）这张多值映射表
    pub fn init_table(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        let _ = tx.open_multimap_table(self.def)?;
        Ok(())
    }
}

/// 专用的 Reader 结构体，它合法地持有 Table，是惰性迭代器的安全基石
pub struct OneToManyReader<K: redb::Key + 'static, V: redb::Key + 'static> {
    table: ReadOnlyMultimapTable<K, V>,
}

impl<K: redb::Key + 'static, V: redb::Key + 'static> OneToManyReader<K, V> {
    /// [Read] 核心修正：直接返回 redb 原生的惰性游标！
    /// 签名极其干净，没有任何多余的内存映射开销
    pub fn get<'a, 'k>(
        &'a self,
        k: impl Borrow<K::SelfType<'k>>,
    ) -> Result<redb::MultimapValue<'a, V>, redb::StorageError> {
        // 直接透传给 redb，拿到纯正的底层惰性游标，它天然实现了 Iterator
        self.table.get(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    // 辅助函数：创建一个纯内存或临时文件的数据库
    fn create_temp_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        Database::create(temp_file.path()).unwrap()
    }

    #[test]
    fn test_crud_one_to_many() {
        let db = create_temp_db();
        // 定义 K=标签(str), V=笔记ID(str)
        let tag_index: OneToMany<&str, &str> = OneToMany::new("tag_to_note");

        // --- 1. 测试写入 (Add) ---
        let write_txn = db.begin_write().unwrap();
        tag_index.add(&write_txn, "Rust", "Note_A").unwrap();
        tag_index.add(&write_txn, "Rust", "Note_B").unwrap();
        tag_index.add(&write_txn, "架构", "Note_C").unwrap();
        write_txn.commit().unwrap();

        // --- 2. 测试读取与惰性迭代器 (Read) ---
        let read_txn = db.begin_read().unwrap();
        let reader = tag_index.reader(&read_txn).unwrap();

        // 拿到游标
        let mut rust_iter = reader.get("Rust").unwrap();

        // 验证多值映射
        let val1 = rust_iter.next().unwrap().unwrap().value().to_string();
        let val2 = rust_iter.next().unwrap().unwrap().value().to_string();
        assert!(val1 == "Note_A" || val1 == "Note_B");
        assert!(val2 == "Note_A" || val2 == "Note_B");
        assert!(rust_iter.next().is_none()); // 没有第三个了

        // --- 3. 测试删除单个关系 (Remove) ---
        let write_txn = db.begin_write().unwrap();
        let removed = tag_index.remove(&write_txn, "Rust", "Note_A").unwrap();
        assert!(removed); // 确认真的删除了
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = tag_index.reader(&read_txn).unwrap();
        let mut rust_iter = reader.get("Rust").unwrap();
        assert_eq!(rust_iter.next().unwrap().unwrap().value(), "Note_B");
        assert!(rust_iter.next().is_none());

        // --- 4. 测试清空该 Key 的所有关系 (Remove All) ---
        let write_txn = db.begin_write().unwrap();
        tag_index.remove_all(&write_txn, "Rust").unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = tag_index.reader(&read_txn).unwrap();
        let mut rust_iter = reader.get("Rust").unwrap();
        assert!(rust_iter.next().is_none()); // 彻底空了
    }

    #[test]
    fn test_reader_outlives_definition() {
        let db = create_temp_db();

        // 写入一点测试数据
        let write_txn = db.begin_write().unwrap();
        let index: OneToMany<&str, &str> = OneToMany::new("temp_index");
        index.add(&write_txn, "Ghost", "Data_1").unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();

        // 见证奇迹的时刻：
        let reader = {
            // 在局部作用域创建一个全新的 OneToMany 定义
            let local_index: OneToMany<&str, &str> = OneToMany::new("temp_index");
            // 生成 reader
            local_index.reader(&read_txn).unwrap()
        }; // <--- local_index 在这里被彻底销毁了！

        // 依然能正常使用 reader 获取底层惰性游标！
        let mut iter = reader.get("Ghost").unwrap();
        assert_eq!(iter.next().unwrap().unwrap().value(), "Data_1");
    }
}
