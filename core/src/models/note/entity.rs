use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NoteRef {
    pub(crate) id: Uuid,
    pub(crate) deleted: bool,
}

#[derive(Clone)]
pub(crate) struct Note {
    pub(crate) id: Uuid,
    pub(crate) deleted: bool,
    pub(crate) inner: NoteBlock,
}

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
        Note::validate_reply_link(tx, &self.id, child_id)?;
        NOTE_LINK.link(tx, &self.id, child_id)
    }

    pub fn link_to_parent(
        self,
        tx: &WriteTransaction,
        parent_id: &Uuid,
    ) -> Result<(), redb::Error> {
        Note::validate_reply_link(tx, parent_id, &self.id)?;
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

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub(crate) fn vector_index() -> VectorStore<Vec<f32>> {
        VectorStore::new("NoteVectors", NOTE_VECTOR_INDEX.dimension())
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

    fn allocate_short_id(tx: &WriteTransaction) -> Result<[u8; 8], redb::Error> {
        loop {
            let short_id = random_id();
            if ID_ALIAS.get_in_write(tx, short_id)?.is_none() {
                return Ok(short_id);
            }
        }
    }

    pub fn create(
        tx: &WriteTransaction,
        content: String,
        tags: Vec<Tag>,
    ) -> Result<Self, redb::Error> {
        let id = Uuid::now_v7();
        let short_id = Self::allocate_short_id(tx)?;
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

    pub fn edit(
        self,
        tx: &WriteTransaction,
        new_content: String,
        tags: Vec<Tag>,
    ) -> Result<Self, redb::Error> {
        self.note_ref().edit(tx, new_content, tags)
    }

    pub fn reply(&self, tx: &WriteTransaction, child: &Note) -> Result<(), redb::Error> {
        self.note_ref().reply_to(tx, &child.id)
    }

    pub fn link_to_parent(&self, tx: &WriteTransaction, parent: &Note) -> Result<(), redb::Error> {
        self.note_ref().link_to_parent(tx, &parent.id)
    }

    pub fn del(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        self.note_ref().del(tx)
    }

    pub fn restore(self, tx: &WriteTransaction) -> Result<(), redb::Error> {
        self.note_ref().restore(tx)
    }

    fn filter_search_text(content: &str) -> String {
        sanitize_search_text(content)
    }

    pub(crate) fn to_version_record(&self) -> NoteVersionRecord {
        NoteVersionRecord {
            id: self.id,
            content: self.inner.content.clone(),
            tags: self.inner.tags.clone(),
        }
    }

    pub(crate) fn import_version(
        tx: &WriteTransaction,
        record: NoteVersionRecord,
    ) -> Result<Self, redb::Error> {
        let note_id = record.id;
        let tag_ids = Self::dedup_tag_ids(record.tags);
        let short_id = Self::allocate_short_id(tx)?;
        let block = NoteBlock {
            content: record.content,
            short_id,
            tags: tag_ids.clone(),
        };

        if let Some(existing) = NOTE_STORE.get_in_write(tx, note_id.as_bytes())? {
            if existing.content != block.content || existing.tags != block.tags {
                return Err(invalid_note_record(
                    "conflicting note version data during import",
                ));
            }

            return Ok(Self {
                id: note_id,
                inner: existing,
                deleted: false,
            });
        }

        NOTE_STORE.put(tx, note_id.as_bytes(), &block)?;

        ID_ALIAS.put(tx, block.short_id, note_id.as_bytes())?;

        for tag_id in &tag_ids {
            let _ = NOTE_TAG_INDEX.add(tx, tag_id.as_bytes(), note_id.as_bytes())?;
        }

        Ok(Self {
            id: note_id,
            inner: block,
            deleted: false,
        })
    }

    pub(crate) fn import_record(
        tx: &WriteTransaction,
        record: NoteRecord,
    ) -> Result<usize, redb::Error> {
        Self::import_records(tx, std::iter::once(record))
    }

    pub(crate) fn import_records(
        tx: &WriteTransaction,
        records: impl IntoIterator<Item = NoteRecord>,
    ) -> Result<usize, redb::Error> {
        let mut unique_record_ids = HashSet::new();
        let mut tags_by_id = BTreeMap::<Uuid, TagSyncRecord>::new();
        let mut notes_by_id = BTreeMap::<Uuid, NoteVersionRecord>::new();
        let mut reply_links = BTreeSet::<ReplyLinkRecord>::new();
        let mut edit_links = BTreeSet::<EditLinkRecord>::new();
        let mut tombstones = BTreeSet::<Uuid>::new();

        for record in records {
            record.validate()?;
            if !unique_record_ids.insert(record.id) {
                continue;
            }

            for tag in record.tags {
                match tags_by_id.entry(tag.id) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(tag);
                    }
                    std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &tag => {
                        return Err(invalid_note_record(
                            "conflicting tag data while importing note records",
                        ));
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {}
                }
            }

            for note in record.notes {
                match notes_by_id.entry(note.id) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(note);
                    }
                    std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &note => {
                        return Err(invalid_note_record(
                            "conflicting note version data while importing note records",
                        ));
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {}
                }
            }

            reply_links.extend(record.reply_links);
            edit_links.extend(record.edit_links);
            tombstones.extend(record.tombstones);
        }

        if unique_record_ids.is_empty() {
            return Ok(0);
        }

        let tag_writer = TagWriter::new(tx);
        for tag in tags_by_id.into_values() {
            tag_writer.import(tag)?;
        }

        for note in notes_by_id.into_values() {
            Self::import_version(tx, note)?;
        }

        for tombstone_id in tombstones {
            Self::import_tombstone(tx, &tombstone_id)?;
        }

        for link in edit_links {
            Self::import_edit_link(tx, &link.previous_id, &link.next_id)?;
        }

        for link in reply_links {
            Self::import_reply_link(tx, &link.parent_id, &link.child_id)?;
        }

        Ok(unique_record_ids.len())
    }

    fn import_reply_link(
        tx: &WriteTransaction,
        parent_id: &Uuid,
        child_id: &Uuid,
    ) -> Result<(), redb::Error> {
        if !Self::exists_in_write(tx, parent_id)? || !Self::exists_in_write(tx, child_id)? {
            return Ok(());
        }

        Self::validate_reply_link(tx, parent_id, child_id)?;
        NOTE_LINK.link(tx, parent_id, child_id)
    }

    fn import_edit_link(
        tx: &WriteTransaction,
        previous_id: &Uuid,
        next_id: &Uuid,
    ) -> Result<(), redb::Error> {
        Self::validate_edit_link(tx, previous_id, next_id)?;
        NOTE_EDIT.link(tx, previous_id, next_id)
    }

    fn import_tombstone(tx: &WriteTransaction, note_id: &Uuid) -> Result<(), redb::Error> {
        NOTE_DELETE.add(tx, note_id.as_bytes()).map(|_| ())
    }

    fn exists_in_write(tx: &WriteTransaction, note_id: &Uuid) -> Result<bool, redb::Error> {
        Ok(NOTE_STORE.get_in_write(tx, note_id.as_bytes())?.is_some())
    }

    fn validate_reply_link(
        tx: &WriteTransaction,
        parent_id: &Uuid,
        child_id: &Uuid,
    ) -> Result<(), redb::Error> {
        if !Self::exists_in_write(tx, parent_id)? || !Self::exists_in_write(tx, child_id)? {
            return Err(invalid_note_record("reply link references a missing note"));
        }

        if NOTE_LINK.would_create_cycle(tx, parent_id, child_id)? {
            return Err(invalid_note_record("reply link would create a cycle"));
        }

        Ok(())
    }

    fn validate_edit_link(
        tx: &WriteTransaction,
        previous_id: &Uuid,
        next_id: &Uuid,
    ) -> Result<(), redb::Error> {
        if !Self::exists_in_write(tx, previous_id)? || !Self::exists_in_write(tx, next_id)? {
            return Err(invalid_note_record(
                "edit link references a missing note version during import",
            ));
        }

        if NOTE_EDIT.would_create_cycle(tx, previous_id, next_id)? {
            return Err(invalid_note_record("edit link would create a cycle"));
        }

        Ok(())
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
