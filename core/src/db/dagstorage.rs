use std::collections::{HashSet, VecDeque};

use crate::db::{
    onetomany::{OneToMany, OneToManyReader},
    types::BlockId,
};
use redb::{ReadTransaction, ReadableMultimapTable, WriteTransaction};
use uuid::Uuid;
// ==========================================
// 1. 静态蓝图：DagStorage
// ==========================================
pub struct DagStore {
    forward: OneToMany<BlockId, BlockId>,
    backward: OneToMany<BlockId, BlockId>,
}

impl DagStore {
    pub const fn new(forward_table: &'static str, backward_table: &'static str) -> Self {
        Self {
            forward: OneToMany::new(forward_table),
            backward: OneToMany::new(backward_table),
        }
    }

    /// 强制在物理磁盘上物化正向和反向两张关系表
    pub fn init_tables(&self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        self.forward.init_table(tx)?;
        self.backward.init_table(tx)?;
        Ok(())
    }

    /// [Write] 写入操作在本地作用域打开 Table，用完即毁，非常安全
    pub fn link(
        &self,
        tx: &WriteTransaction,
        parent: &Uuid,
        child: &Uuid,
    ) -> Result<(), redb::Error> {
        let p_bytes = parent.into_bytes();
        let c_bytes = child.into_bytes();
        self.forward.add(tx, p_bytes, c_bytes)?;
        self.backward.add(tx, c_bytes, p_bytes)?;
        Ok(())
    }

    pub fn unlink(
        &self,
        tx: &WriteTransaction,
        parent: &Uuid,
        child: &Uuid,
    ) -> Result<(), redb::Error> {
        let p_bytes = parent.into_bytes();
        let c_bytes = child.into_bytes();
        self.forward.remove(tx, p_bytes, c_bytes)?;
        self.backward.remove(tx, c_bytes, p_bytes)?;
        Ok(())
    }

    pub fn would_create_cycle(
        &self,
        tx: &WriteTransaction,
        parent: &Uuid,
        child: &Uuid,
    ) -> Result<bool, redb::Error> {
        if parent == child {
            return Ok(true);
        }

        self.path_exists_in_write(tx, child, parent)
    }

    /// [Read] 核心修正：交出 Reader，而不是越权返回 Iterator
    pub fn reader(&self, tx: &ReadTransaction) -> Result<DagReader, redb::Error> {
        Ok(DagReader {
            forward: self.forward.reader(tx)?,
            backward: self.backward.reader(tx)?,
        })
    }

    fn path_exists_in_write(
        &self,
        tx: &WriteTransaction,
        start: &Uuid,
        target: &Uuid,
    ) -> Result<bool, redb::Error> {
        let table = tx.open_multimap_table(self.forward.table_def())?;
        let start_bytes = start.into_bytes();
        let target_bytes = target.into_bytes();
        let mut queue = VecDeque::from([start_bytes]);
        let mut visited = HashSet::from([start_bytes]);

        while let Some(current) = queue.pop_front() {
            let children = table.get(current)?;
            for child_res in children {
                let child: BlockId = child_res?.value();
                if child == target_bytes {
                    return Ok(true);
                }

                if visited.insert(child) {
                    queue.push_back(child);
                }
            }
        }

        Ok(false)
    }
}

// ==========================================
// 2. 状态游标：DagReader
// 负责在内存中稳稳地持有两张 Table，给 Iterator 兜底！
// ==========================================
pub struct DagReader {
    forward: OneToManyReader<BlockId, BlockId>,
    backward: OneToManyReader<BlockId, BlockId>,
}

impl DagReader {
    /// 现在的 'a 生命期直接绑定到 &'a self 上！
    /// 只要外部调用者不销毁 DagReader，这个惰性迭代器就绝对安全。
    pub fn get_children<'a>(
        &'a self,
        parent: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + 'a, redb::StorageError>
    {
        let p_bytes = parent.into_bytes();

        // iter 借用了 self.forward
        let iter = self.forward.get(p_bytes)?;

        Ok(iter.map(|guard_res| guard_res.map(|guard| Uuid::from_bytes(guard.value()))))
    }

    pub fn get_parents<'a>(
        &'a self,
        child: &Uuid,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + 'a, redb::StorageError>
    {
        let c_bytes = child.into_bytes();
        let iter = self.backward.get(c_bytes)?;

        Ok(iter.map(|guard_res| guard_res.map(|guard| Uuid::from_bytes(guard.value()))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use std::collections::HashSet;
    use tempfile::NamedTempFile;

    // 辅助函数：创建一个纯内存或临时文件的数据库
    fn create_temp_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        Database::create(temp_file.path()).unwrap()
    }

    // 辅助函数：将迭代器收集为 HashSet，方便进行无序的等价断言
    fn collect_uuids(
        iter: impl Iterator<Item = Result<Uuid, redb::StorageError>>,
    ) -> HashSet<Uuid> {
        iter.map(|res| res.unwrap()).collect()
    }

    #[test]
    fn test_dag_basic_link_and_read() {
        let db = create_temp_db();
        let dag = DagStore::new("fwd_basic", "bwd_basic");

        let node_a = Uuid::new_v4(); // 父节点
        let node_b = Uuid::new_v4(); // 子节点 1
        let node_c = Uuid::new_v4(); // 子节点 2

        // 1. 写入拓扑关系: A -> B, A -> C
        let write_txn = db.begin_write().unwrap();
        dag.link(&write_txn, &node_a, &node_b).unwrap();
        dag.link(&write_txn, &node_a, &node_c).unwrap();
        write_txn.commit().unwrap();

        // 2. 读取验证 (正向：寻找派生的子节点)
        let read_txn = db.begin_read().unwrap();
        let reader = dag.reader(&read_txn).unwrap();

        let children_of_a = collect_uuids(reader.get_children(&node_a).unwrap());
        assert_eq!(children_of_a.len(), 2);
        assert!(children_of_a.contains(&node_b));
        assert!(children_of_a.contains(&node_c));

        // 3. 读取验证 (反向：向上溯源寻找父节点)
        let parents_of_b = collect_uuids(reader.get_parents(&node_b).unwrap());
        assert_eq!(parents_of_b.len(), 1);
        assert!(parents_of_b.contains(&node_a));
    }

    #[test]
    fn test_dag_unlink() {
        let db = create_temp_db();
        let dag = DagStore::new("fwd_unlink", "bwd_unlink");

        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        // 建立连接
        let write_txn = db.begin_write().unwrap();
        dag.link(&write_txn, &parent, &child).unwrap();
        write_txn.commit().unwrap();

        // 斩断连接
        let write_txn = db.begin_write().unwrap();
        dag.unlink(&write_txn, &parent, &child).unwrap();
        write_txn.commit().unwrap();

        // 验证连接是否彻底在双端消失
        let read_txn = db.begin_read().unwrap();
        let reader = dag.reader(&read_txn).unwrap();

        let children = collect_uuids(reader.get_children(&parent).unwrap());
        assert!(children.is_empty());

        let parents = collect_uuids(reader.get_parents(&child).unwrap());
        assert!(parents.is_empty());
    }

    #[test]
    fn test_dag_complex_diamond_topology() {
        let db = create_temp_db();
        let dag = DagStore::new("fwd_diamond", "bwd_diamond");

        // 构造一个经典的菱形 (Diamond) 拓扑:
        // 多对一与一对多的完美混合
        //   A
        //  / \
        // B   C
        //  \ /
        //   D
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();

        let write_txn = db.begin_write().unwrap();
        dag.link(&write_txn, &a, &b).unwrap();
        dag.link(&write_txn, &a, &c).unwrap();
        dag.link(&write_txn, &b, &d).unwrap();
        dag.link(&write_txn, &c, &d).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = dag.reader(&read_txn).unwrap();

        // 验证多对一：D 的父节点应该是 B 和 C
        let parents_of_d = collect_uuids(reader.get_parents(&d).unwrap());
        assert_eq!(parents_of_d.len(), 2);
        assert!(parents_of_d.contains(&b));
        assert!(parents_of_d.contains(&c));

        // 验证一对多：A 的子节点应该是 B 和 C
        let children_of_a = collect_uuids(reader.get_children(&a).unwrap());
        assert_eq!(children_of_a.len(), 2);
        assert!(children_of_a.contains(&b));
        assert!(children_of_a.contains(&c));
    }
}
