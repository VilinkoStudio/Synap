use super::*;

pub(crate) struct NoteReader<'a> {
    tx: &'a ReadTransaction,
    note_table: KvReader<BlockId, NoteBlock>,
    alias_table: KvReader<[u8; 8], BlockId>,
    link_dag: DagReader,
    edit_dag: DagReader,
    del_set: SetReader<BlockId>,
    tag_index: OneToManyReader<BlockId, BlockId>,
}

impl<'a> NoteReader<'a> {
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

    pub fn tx(&self) -> &'a ReadTransaction {
        self.tx
    }

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
        self.note_by_time_range(Bound::Unbounded, Bound::Unbounded)
    }

    pub fn note_by_time_range(
        &self,
        start: Bound<Uuid>,
        end: Bound<Uuid>,
    ) -> Result<
        impl DoubleEndedIterator<Item = Result<Uuid, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let id_iter = self
            .note_table
            .keys_range((uuid_bound_to_block_id(start), uuid_bound_to_block_id(end)))?;
        Ok(id_iter.map(|item| item.map(|key_guard| Uuid::from_bytes(key_guard.value()))))
    }

    pub fn is_deleted(&self, id: &Uuid) -> Result<bool, redb::Error> {
        self.del_set.contains(id.as_bytes())
    }

    pub fn tagged_note_ids(
        &self,
        tag: &Tag,
    ) -> Result<impl Iterator<Item = Result<Uuid, redb::StorageError>> + '_, redb::Error> {
        let iter = self.tag_index.get(tag.get_id().as_bytes())?;
        Ok(iter.map(|guard_res| guard_res.map(|guard| Uuid::from_bytes(guard.value()))))
    }

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
        self.deleted_note_ids_range(Bound::Unbounded, Bound::Unbounded)
    }

    pub fn deleted_note_ids_range(
        &self,
        start: Bound<Uuid>,
        end: Bound<Uuid>,
    ) -> Result<
        impl DoubleEndedIterator<Item = Result<Uuid, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self
            .del_set
            .range((uuid_bound_to_block_id(start), uuid_bound_to_block_id(end)))?;
        Ok(iter.map(|item| item.map(|(guard, _)| Uuid::from_bytes(guard.value()))))
    }

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

    pub(crate) fn export_record(&self, id: &Uuid) -> Result<Option<NoteRecord>, redb::Error> {
        if self.get_ref_by_id(id)?.is_none() {
            return Ok(None);
        }

        let version_ids = self.logical_note_version_ids(id)?;
        let version_id_set: HashSet<_> = version_ids.iter().copied().collect();
        let tag_reader = TagReader::new(self.tx)?;

        let notes = version_ids
            .iter()
            .map(|version_id| {
                self.get_by_id(version_id)?
                    .ok_or_else(|| invalid_note_record("missing note version during export"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut note_records: Vec<_> = notes.iter().map(Note::to_version_record).collect();
        note_records.sort_by_key(|note| note.id);

        let mut tag_ids = BTreeSet::new();
        let mut tombstones = Vec::new();
        for note in &notes {
            tag_ids.extend(note.tags().iter().copied());
            if note.is_deleted() {
                tombstones.push(note.get_id());
            }
        }

        let mut tags = Vec::with_capacity(tag_ids.len());
        for tag_id in tag_ids {
            let tag = tag_reader
                .get_by_id(&tag_id)?
                .ok_or_else(|| invalid_note_record("missing tag during note record export"))?;
            tags.push(tag.to_sync_record());
        }
        tags.sort_by_key(|tag| tag.id);

        let mut edit_links = BTreeSet::new();
        let mut reply_links = BTreeSet::new();

        for version_id in &version_ids {
            for next_id in self.next_versions_raw(version_id)? {
                let next_id = next_id?;
                if version_id_set.contains(&next_id) {
                    edit_links.insert(EditLinkRecord {
                        previous_id: *version_id,
                        next_id,
                    });
                }
            }

            for child_id in self.children_raw(version_id)? {
                reply_links.insert(ReplyLinkRecord {
                    parent_id: *version_id,
                    child_id: child_id?,
                });
            }

            for parent_id in self.parents_raw(version_id)? {
                reply_links.insert(ReplyLinkRecord {
                    parent_id: parent_id?,
                    child_id: *version_id,
                });
            }
        }

        tombstones.sort_unstable();

        Ok(Some(NoteRecord {
            id: version_ids[0],
            notes: note_records,
            tags,
            reply_links: reply_links.into_iter().collect(),
            edit_links: edit_links.into_iter().collect(),
            tombstones,
        }))
    }

    pub(crate) fn export_records(&self, note_ids: &[Uuid]) -> Result<Vec<NoteRecord>, redb::Error> {
        let mut seen = HashSet::new();
        let mut records = Vec::new();

        for note_id in note_ids {
            if let Some(record) = self.export_record(note_id)? {
                if seen.insert(record.id) {
                    records.push(record);
                }
            }
        }

        records.sort_by_key(|record| record.id);
        Ok(records)
    }

    fn logical_note_version_ids(&self, id: &Uuid) -> Result<Vec<Uuid>, redb::Error> {
        let mut version_ids = std::iter::once(*id)
            .chain(
                self.all_versions_raw(id)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(redb::Error::from)?,
            )
            .collect::<Vec<_>>();

        version_ids.sort_unstable();
        version_ids.dedup();
        Ok(version_ids)
    }
}

fn uuid_bound_to_block_id(bound: Bound<Uuid>) -> Bound<BlockId> {
    match bound {
        Bound::Included(uuid) => Bound::Included(uuid.into_bytes()),
        Bound::Excluded(uuid) => Bound::Excluded(uuid.into_bytes()),
        Bound::Unbounded => Bound::Unbounded,
    }
}
