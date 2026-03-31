use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    dto::NoteDTO,
    error::NoteError,
    models::{
        note::{Note, NoteReader},
        tag::TagReader,
    },
};

/// NoteView：零成本的业务视图包装
/// 'a = NoteReader 的借用生命周期
/// 'b = NoteReader 内部 transaction 的生命周期
pub struct NoteView<'a, 'b: 'a> {
    reader: &'a NoteReader<'b>,
    pub note: Note,
}

impl<'a, 'b> NoteView<'a, 'b> {
    /// 构造一个视图导航器（零成本，只是借用 Reader）
    pub fn new(reader: &'a NoteReader<'b>, note: Note) -> Self {
        Self { reader, note }
    }

    /// 从 ID 构造 View
    pub fn from_id(reader: &'a NoteReader<'b>, id: Uuid) -> Result<Self, NoteError> {
        let note = reader
            .get_by_id(&id)
            .map_err(|err| NoteError::Db(err))?
            .ok_or(NoteError::IdNotFound { id })?;
        Ok(Self::new(reader, note))
    }

    pub fn from_short_id(reader: &'a NoteReader<'b>, id: [u8; 8]) -> Result<Self, NoteError> {
        let note = reader
            .get_by_short_id(&id)
            .map_err(|err| NoteError::Db(err))?
            .ok_or(NoteError::ShortIdNotFound { id })?;
        Ok(Self::new(reader, note))
    }

    /// UUID 转换为 NoteView（内部方法）
    fn uuid_to_view(
        &'a self,
        iter: impl Iterator<Item = Result<Uuid, NoteError>> + 'a,
    ) -> impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a {
        iter.filter_map(move |uuid_res| match uuid_res {
            Ok(id) => match self.reader.get_by_id(&id) {
                Ok(Some(note)) if !note.is_deleted() => Some(Ok(NoteView::new(self.reader, note))),
                Ok(Some(_)) => None, // 过滤已删除
                Ok(None) => Some(Err(NoteError::IdNotFound { id })),
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(e)),
        })
    }

    /// 向上溯源：返回父节点的迭代器（包含所有版本的父节点）
    /// 如果遇到已删除的节点，继续向上穿透其 parents
    pub fn parents(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a, NoteError> {
        let version_ids: Vec<Uuid> = std::iter::once(self.note.get_id())
            .chain(
                self.reader
                    .all_versions(&self.note)
                    .map_err(|e| NoteError::Db(e.into()))?
                    .filter_map(Result::ok),
            )
            .collect();

        let mut seen: HashSet<Uuid> = version_ids.iter().copied().collect();
        let mut queue: Vec<Uuid> = Vec::new();
        let mut result = Vec::new();

        // 初始：收集所有版本的直接 parents
        for version_id in version_ids {
            let parent_ids = self
                .reader
                .parents_raw(&version_id)
                .map_err(|e| NoteError::Db(e.into()))?;
            for parent_res in parent_ids {
                let parent_id = parent_res.map_err(|e| NoteError::Db(e.into()))?;
                if seen.insert(parent_id) {
                    queue.push(parent_id);
                }
            }
        }

        // BFS 穿透已删除节点
        let mut i = 0;
        while i < queue.len() {
            let id = queue[i];
            i += 1;

            match self.reader.get_by_id(&id) {
                Ok(Some(note)) if !note.is_deleted() => {
                    result.push(Ok(id));
                }
                Ok(Some(_)) => {
                    // 已删除：穿透，继续向上找它的 parents
                    if let Ok(parent_ids) = self.reader.parents_raw(&id) {
                        for parent_res in parent_ids {
                            if let Ok(parent_id) = parent_res {
                                if seen.insert(parent_id) {
                                    queue.push(parent_id);
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    result.push(Err(NoteError::IdNotFound { id }));
                }
                Err(e) => {
                    result.push(Err(NoteError::Db(e.into())));
                }
            }
        }

        Ok(self.uuid_to_view(result.into_iter()))
    }

    /// 向下推演：返回子节点的迭代器（包含所有版本的子节点）
    /// 如果遇到已删除的节点，继续向下穿透其 children
    pub fn children(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a, NoteError> {
        let version_ids: Vec<Uuid> = std::iter::once(self.note.get_id())
            .chain(
                self.reader
                    .all_versions(&self.note)
                    .map_err(|e| NoteError::Db(e.into()))?
                    .filter_map(Result::ok),
            )
            .collect();

        let mut seen: HashSet<Uuid> = version_ids.iter().copied().collect();
        let mut queue: Vec<Uuid> = Vec::new();
        let mut result = Vec::new();

        // 初始：收集所有版本的直接 children
        for version_id in version_ids {
            let child_ids = self
                .reader
                .children_raw(&version_id)
                .map_err(|e| NoteError::Db(e.into()))?;
            for child_res in child_ids {
                let child_id = child_res.map_err(|e| NoteError::Db(e.into()))?;
                if seen.insert(child_id) {
                    queue.push(child_id);
                }
            }
        }

        // BFS 穿透已删除节点
        let mut i = 0;
        while i < queue.len() {
            let id = queue[i];
            i += 1;

            match self.reader.get_by_id(&id) {
                Ok(Some(note)) if !note.is_deleted() => {
                    result.push(Ok(id));
                }
                Ok(Some(_)) => {
                    // 已删除：穿透，继续向下找它的 children
                    if let Ok(child_ids) = self.reader.children_raw(&id) {
                        for child_res in child_ids {
                            if let Ok(child_id) = child_res {
                                if seen.insert(child_id) {
                                    queue.push(child_id);
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    result.push(Err(NoteError::IdNotFound { id }));
                }
                Err(e) => {
                    result.push(Err(NoteError::Db(e.into())));
                }
            }
        }

        Ok(self.uuid_to_view(result.into_iter()))
    }

    /// 获取历史版本沿革（过滤已删除）
    pub fn history(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a, NoteError> {
        let iter = self
            .reader
            .previous_versions(&self.note)
            .map_err(|e| NoteError::Db(e.into()))?;
        let aligned = iter.map(|res| res.map_err(|e| NoteError::Db(e.into())));
        Ok(self.uuid_to_view(aligned))
    }

    /// 获取下一个版本（过滤已删除）
    pub fn next_version(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a, NoteError> {
        let iter = self
            .reader
            .next_versions(&self.note)
            .map_err(|e| NoteError::Db(e.into()))?;
        let aligned = iter.map(|res| res.map_err(|e| NoteError::Db(e.into())));
        Ok(self.uuid_to_view(aligned))
    }

    /// 获取当前版本在编辑链上的其他版本（过滤已删除）
    pub fn other_versions(
        &'a self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'a, 'b>, NoteError>> + 'a, NoteError> {
        let iter = self
            .reader
            .other_versions(&self.note)
            .map_err(|e| NoteError::Db(e.into()))?;
        let aligned = iter.map(|res| res.map_err(|e| NoteError::Db(e.into())));
        Ok(self.uuid_to_view(aligned))
    }

    /// 获取当前节点的标签
    pub fn tags(&self) -> Result<Vec<crate::models::tag::Tag>, NoteError> {
        let tag_reader = TagReader::new(self.reader.tx()).map_err(|e| NoteError::Db(e))?;
        self.note
            .tags()
            .iter()
            .map(|id| {
                tag_reader
                    .get_by_id(id)
                    .map_err(|err| NoteError::Db(err.into()))?
                    .ok_or_else(|| NoteError::IdNotFound { id: *id })
            })
            .collect()
    }

    /// 组装 DTO
    pub fn to_dto(&self) -> Result<NoteDTO, NoteError> {
        let tags: Vec<String> = self
            .tags()?
            .iter()
            .map(|t| t.get_content().to_string())
            .collect();
        let (seconds, nanos) = self
            .note
            .get_id()
            .get_timestamp()
            .ok_or(NoteError::IdNotFound {
                id: self.note.get_id(),
            })?
            .to_unix();

        Ok(NoteDTO {
            id: self.note.get_id().to_string(),
            content: self.note.content().to_string(),
            tags,
            created_at: seconds.saturating_mul(1000) + u64::from(nanos / 1_000_000),
        })
    }

    pub fn get_note(&self) -> &Note {
        &self.note
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
        crate::models::note::Note::init_schema(&write_txn).expect("Failed to initialize schema");
        crate::models::tag::TagWriter::init_schema(&write_txn)
            .expect("Failed to initialize tag schema");
        write_txn.commit().unwrap();
        db
    }
    // 测试一系列的惰性迭代器 处理复杂的dag关系
    #[test]
    fn test_note_view_parents_includes_all_version_parents() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let v0 = Note::create(&write_txn, "v0".to_string(), vec![]).unwrap();
        let v1 = v0
            .clone()
            .edit(&write_txn, "v1".to_string(), vec![])
            .unwrap();
        let v1_id = v1.get_id();

        let parent_of_v0 = Note::create(&write_txn, "parent_of_v0".to_string(), vec![]).unwrap();
        let parent_of_v0_id = parent_of_v0.get_id();
        parent_of_v0.reply(&write_txn, &v0).unwrap();

        let parent_of_v1 = Note::create(&write_txn, "parent_of_v1".to_string(), vec![]).unwrap();
        let parent_of_v1_id = parent_of_v1.get_id();
        parent_of_v1.reply(&write_txn, &v1).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let v1_view = NoteView::new(&reader, reader.get_by_id(&v1_id).unwrap().unwrap());
        let parents: Vec<Uuid> = v1_view
            .parents()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        assert!(parents.contains(&parent_of_v0_id));
        assert!(parents.contains(&parent_of_v1_id));
    }

    #[test]
    fn test_note_view_children_includes_all_version_children() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let parent = Note::create(&write_txn, "parent".to_string(), vec![]).unwrap();

        let v1 = parent
            .clone()
            .edit(&write_txn, "v1".to_string(), vec![])
            .unwrap();
        let v1_id = v1.get_id();

        let child1 = Note::create(&write_txn, "child1".to_string(), vec![]).unwrap();
        let child1_id = child1.get_id();
        parent.reply(&write_txn, &child1).unwrap();

        let child2 = Note::create(&write_txn, "child2".to_string(), vec![]).unwrap();
        let child2_id = child2.get_id();
        v1.reply(&write_txn, &child2).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let v1_view = NoteView::new(&reader, reader.get_by_id(&v1_id).unwrap().unwrap());
        let children: Vec<Uuid> = v1_view
            .children()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        assert!(
            children.contains(&child1_id),
            "should contain child1 (from parent version)"
        );
        assert!(
            children.contains(&child2_id),
            "should contain child2 (from v1 version)"
        );
    }

    #[test]
    fn test_note_view_parents_deduplicates() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let v0 = Note::create(&write_txn, "v0".to_string(), vec![]).unwrap();
        let v1 = v0
            .clone()
            .edit(&write_txn, "v1".to_string(), vec![])
            .unwrap();
        let v1_id = v1.get_id();

        let shared_parent = Note::create(&write_txn, "shared_parent".to_string(), vec![]).unwrap();
        let shared_parent_id = shared_parent.get_id();
        shared_parent.reply(&write_txn, &v0).unwrap();
        shared_parent.reply(&write_txn, &v1).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let v1_view = NoteView::new(&reader, reader.get_by_id(&v1_id).unwrap().unwrap());
        let parents: Vec<Uuid> = v1_view
            .parents()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        let parent_count = parents.iter().filter(|&&id| id == shared_parent_id).count();
        assert_eq!(parent_count, 1, "shared parent should appear only once");
    }

    #[test]
    fn test_note_view_children_deduplicates() {
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let parent = Note::create(&write_txn, "parent".to_string(), vec![]).unwrap();

        let v1 = parent
            .clone()
            .edit(&write_txn, "v1".to_string(), vec![])
            .unwrap();
        let v1_id = v1.get_id();

        let child = Note::create(&write_txn, "child".to_string(), vec![]).unwrap();
        let child_id = child.get_id();
        parent.reply(&write_txn, &child).unwrap();
        v1.reply(&write_txn, &child).unwrap();
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let v1_view = NoteView::new(&reader, reader.get_by_id(&v1_id).unwrap().unwrap());
        let children: Vec<Uuid> = v1_view
            .children()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        let child_count = children.iter().filter(|&&id| id == child_id).count();
        assert_eq!(child_count, 1, "child should appear only once");
    }

    #[test]
    fn test_parents_penetrates_deleted_node() {
        // grandparent -> [deleted middle] -> note
        // parents(note) should return grandparent (skipping deleted middle)
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let gp = Note::create(&write_txn, "grandparent".to_string(), vec![]).unwrap();
        let gp_id = gp.get_id();

        let middle = Note::create(&write_txn, "middle".to_string(), vec![]).unwrap();
        let middle_id = middle.get_id();

        let note = Note::create(&write_txn, "note".to_string(), vec![]).unwrap();
        let note_id = note.get_id();

        // grandparent -> middle (deleted)
        gp.reply(&write_txn, &middle).unwrap();
        // middle (deleted) -> note
        middle.reply(&write_txn, &note).unwrap();

        // 删除 middle
        middle.del(&write_txn).unwrap();

        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let note_view = NoteView::new(&reader, reader.get_by_id(&note_id).unwrap().unwrap());
        let parents: Vec<Uuid> = note_view
            .parents()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        // middle 被删除，应该穿透到 grandparent
        assert_eq!(
            parents.len(),
            1,
            "should have exactly 1 parent after penetration"
        );
        assert!(parents.contains(&gp_id), "should contain grandparent");
        assert!(
            !parents.contains(&middle_id),
            "should not contain deleted middle"
        );
    }

    #[test]
    fn test_children_penetrates_deleted_node() {
        // note -> [deleted middle] -> grandchild
        // children(note) should return grandchild (skipping deleted middle)
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let note = Note::create(&write_txn, "note".to_string(), vec![]).unwrap();
        let note_id = note.get_id();

        let middle = Note::create(&write_txn, "middle".to_string(), vec![]).unwrap();
        let middle_id = middle.get_id();

        let gc = Note::create(&write_txn, "grandchild".to_string(), vec![]).unwrap();
        let gc_id = gc.get_id();

        // note -> middle (deleted)
        note.reply(&write_txn, &middle).unwrap();
        // middle (deleted) -> grandchild
        middle.reply(&write_txn, &gc).unwrap();

        // 删除 middle
        middle.del(&write_txn).unwrap();

        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let note_view = NoteView::new(&reader, reader.get_by_id(&note_id).unwrap().unwrap());
        let children: Vec<Uuid> = note_view
            .children()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        // middle 被删除，应该穿透到 grandchild
        assert_eq!(
            children.len(),
            1,
            "should have exactly 1 child after penetration"
        );
        assert!(children.contains(&gc_id), "should contain grandchild");
        assert!(
            !children.contains(&middle_id),
            "should not contain deleted middle"
        );
    }

    #[test]
    fn test_penetration_does_not_follow_infinite_loop() {
        // 验证穿透不会死循环（deleted node 指回已访问节点时）
        // A -> [deleted B] -> [deleted C] -> D
        // 且 B 的 parents 也包含 A（形成回路）
        let db = create_temp_db();

        let write_txn = db.begin_write().unwrap();
        let a = Note::create(&write_txn, "A".to_string(), vec![]).unwrap();
        let a_id = a.get_id();
        let b = Note::create(&write_txn, "B".to_string(), vec![]).unwrap();
        let b_id = b.get_id();
        let c = Note::create(&write_txn, "C".to_string(), vec![]).unwrap();
        let c_id = c.get_id();
        let d = Note::create(&write_txn, "D".to_string(), vec![]).unwrap();
        let d_id = d.get_id();

        // A -> B (deleted) -> C (deleted) -> D
        a.reply(&write_txn, &b).unwrap();
        b.reply(&write_txn, &c).unwrap();
        c.reply(&write_txn, &d).unwrap();

        // 回路：B 也连回 A
        b.reply(&write_txn, &a).unwrap();

        b.del(&write_txn).unwrap();
        c.del(&write_txn).unwrap();

        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let reader = NoteReader::new(&read_txn).unwrap();

        let a_view = NoteView::new(&reader, reader.get_by_id(&a_id).unwrap().unwrap());
        let children: Vec<Uuid> = a_view
            .children()
            .unwrap()
            .map(|item| item.unwrap().get_note().get_id())
            .collect();

        // 穿透 B(deleted) -> C(deleted) -> D(alive)
        assert_eq!(
            children.len(),
            1,
            "children should be [D] only, got {:?}",
            children
        );
        assert!(children.contains(&d_id));
        assert!(!children.contains(&b_id));
        assert!(!children.contains(&c_id));
        // A 自己不应该出现在 children 里（即使穿透时回到 A，A 已 visited）
        assert!(!children.contains(&a_id));
    }
}
