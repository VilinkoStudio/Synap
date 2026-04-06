use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoteVersionRecord {
    pub id: Uuid,
    pub content: String,
    pub short_id: [u8; 8],
    pub tags: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReplyLinkRecord {
    pub parent_id: Uuid,
    pub child_id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EditLinkRecord {
    pub previous_id: Uuid,
    pub next_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteRecord {
    pub id: Uuid,
    pub notes: Vec<NoteVersionRecord>,
    pub tags: Vec<TagSyncRecord>,
    pub reply_links: Vec<ReplyLinkRecord>,
    pub edit_links: Vec<EditLinkRecord>,
    pub tombstones: Vec<Uuid>,
}

impl NoteRecord {
    pub fn sync_id(&self) -> Result<Uuid, postcard::Error> {
        let namespace = Uuid::new_v5(&Uuid::NAMESPACE_OID, b"synap.note-record.sync");
        let payload = postcard::to_allocvec(self)?;
        Ok(Uuid::new_v5(&namespace, &payload))
    }

    pub(crate) fn validate(&self) -> Result<(), redb::Error> {
        if self.notes.is_empty() {
            return Err(invalid_note_record("note record cannot be empty"));
        }

        let mut note_ids = HashSet::new();
        for note in &self.notes {
            if !note_ids.insert(note.id) {
                return Err(invalid_note_record("duplicate note version in note record"));
            }
        }

        let expected_id = self.notes.iter().map(|note| note.id).min().unwrap();
        if self.id != expected_id {
            return Err(invalid_note_record(
                "note record id must be the minimum version id in the logical note",
            ));
        }

        let mut tag_ids = HashSet::new();
        for tag in &self.tags {
            if !tag_ids.insert(tag.id) {
                return Err(invalid_note_record("duplicate tag in note record"));
            }
        }

        for note in &self.notes {
            for tag_id in &note.tags {
                if !tag_ids.contains(tag_id) {
                    return Err(invalid_note_record(
                        "note record is missing a tag referenced by a note version",
                    ));
                }
            }
        }

        for tombstone_id in &self.tombstones {
            if !note_ids.contains(tombstone_id) {
                return Err(invalid_note_record(
                    "tombstone references a note outside the logical note",
                ));
            }
        }

        for link in &self.edit_links {
            if !note_ids.contains(&link.previous_id) || !note_ids.contains(&link.next_id) {
                return Err(invalid_note_record(
                    "edit link references a note outside the logical note",
                ));
            }
        }

        for link in &self.reply_links {
            if !note_ids.contains(&link.parent_id) && !note_ids.contains(&link.child_id) {
                return Err(invalid_note_record(
                    "reply link must touch at least one note inside the logical note",
                ));
            }
        }

        Ok(())
    }
}
