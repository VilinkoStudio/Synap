//! Type-state note drafts. Local-only staging before ledger append; no derived indexes.
//!
//! Flow: Memory (process-local) → optional `persist` → redb → `commit` emits a note and a
//! [`DraftCommitReceipt`]. Edges are *intent* here (`origin` / `reply_to`); real
//! `NOTE_EDIT` / `NOTE_LINK` edges are written only at commit via `AppendNoteCommand`.

use crate::{
    db::{kvstore::KvStore, types::BlockId},
    models::tag_metadata::{
        parse_color_input, NoteColor, TagMetadata, TagMetadataError, UnknownMeta,
    },
};
use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    marker::PhantomData,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const DRAFT_STORE: KvStore<BlockId, StoredDraftRecord> = KvStore::new("NoteDraftsV1");
const DRAFT_COMMIT_RECEIPTS: KvStore<BlockId, DraftCommitReceipt> =
    KvStore::new("NoteDraftCommitReceiptsV1");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct DraftId(Uuid);

impl DraftId {
    pub(crate) fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub(crate) fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub(crate) fn as_uuid(self) -> Uuid {
        self.0
    }

    fn as_key(&self) -> &BlockId {
        self.0.as_bytes()
    }
}

/// Edit lineage intent for a draft. Orthogonal to [`DraftData::reply_to`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DraftOrigin {
    New,
    /// Commit should create a new version of `base` (`NOTE_EDIT`).
    Edit { base: Uuid },
}

/// Editable draft payload. Display tags and structured metadata are kept separate;
/// they are merged into storage tags only when building a `NoteSnapshot` at commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DraftData {
    id: DraftId,
    content: String,
    /// User-facing tags only (no `$meta` forms).
    tags: Vec<String>,
    color: Option<NoteColor>,
    /// Preserved `$kind(...)` entries so edit round-trips do not drop unknown kinds.
    unknown_meta: Vec<UnknownMeta>,
    origin: DraftOrigin,
    /// Optional reply parent; may combine with [`DraftOrigin::Edit`] (EditReply).
    reply_to: Option<Uuid>,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl DraftData {
    fn new() -> Self {
        let now = now_ms();
        Self {
            id: DraftId::new(),
            content: String::new(),
            tags: Vec::new(),
            color: None,
            unknown_meta: Vec::new(),
            origin: DraftOrigin::New,
            reply_to: None,
            created_at_ms: now,
            updated_at_ms: now,
        }
    }

    pub(crate) fn id(&self) -> DraftId {
        self.id
    }
    pub(crate) fn content(&self) -> &str {
        &self.content
    }
    pub(crate) fn tags(&self) -> &[String] {
        &self.tags
    }
    pub(crate) fn color(&self) -> Option<NoteColor> {
        self.color
    }
    pub(crate) fn origin(&self) -> &DraftOrigin {
        &self.origin
    }
    pub(crate) fn reply_to(&self) -> Option<Uuid> {
        self.reply_to
    }
    pub(crate) fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }
    pub(crate) fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    pub(crate) fn set_content(&mut self, content: String) {
        self.content = content;
        self.touch();
    }

    pub(crate) fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.touch();
    }

    pub(crate) fn set_color(&mut self, color: Option<&str>) -> Result<(), TagMetadataError> {
        self.color = parse_color_input(color)?;
        self.touch();
        Ok(())
    }

    pub(crate) fn set_reply_to(&mut self, reply_to: Option<Uuid>) {
        self.reply_to = reply_to;
        self.touch();
    }

    pub(crate) fn set_origin(&mut self, origin: DraftOrigin) {
        self.origin = origin;
        self.touch();
    }

    pub(crate) fn set_unknown_meta(&mut self, unknown_meta: Vec<UnknownMeta>) {
        self.unknown_meta = unknown_meta;
        self.touch();
    }

    pub(crate) fn metadata(&self) -> TagMetadata {
        TagMetadata {
            color: self.color,
            unknown: self.unknown_meta.clone(),
        }
    }

    fn touch(&mut self) {
        self.updated_at_ms = now_ms();
    }
}

/// Process-local draft: no optimistic concurrency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Memory;

/// redb-backed draft; `revision` is the optimistic-concurrency token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Persisted {
    revision: u64,
}

/// Type-state wrapper: [`NoteDraftMemory`] vs [`PersistedNoteDraft`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteDraft<S> {
    data: DraftData,
    state: S,
}

pub(crate) type NoteDraftMemory = NoteDraft<Memory>;
pub(crate) type PersistedNoteDraft = NoteDraft<Persisted>;

impl NoteDraftMemory {
    pub(crate) fn new() -> Self {
        Self {
            data: DraftData::new(),
            state: Memory,
        }
    }
}

impl PersistedNoteDraft {
    fn from_record(record: StoredDraftRecord) -> Self {
        Self {
            data: record.data,
            state: Persisted {
                revision: record.revision,
            },
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.state.revision
    }
}

impl<S> NoteDraft<S> {
    pub(crate) fn data(&self) -> &DraftData {
        &self.data
    }
    pub(crate) fn data_mut(&mut self) -> &mut DraftData {
        &mut self.data
    }
    pub(crate) fn into_data(self) -> DraftData {
        self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredDraftRecord {
    data: DraftData,
    revision: u64,
}

/// Idempotency record: `draft_id → committed note` after a successful commit.
/// Survives draft deletion so `draft_commit` retries return the same note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DraftCommitReceipt {
    pub(crate) note_id: Uuid,
    /// Needed so receipt retries still take the edit branch of `finish_note_append`.
    pub(crate) edited_from: Option<Uuid>,
}

pub(crate) struct DraftRepository;

impl DraftRepository {
    pub(crate) fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        DRAFT_STORE.init_table(tx)?;
        DRAFT_COMMIT_RECEIPTS.init_table(tx)?;
        Ok(())
    }

    pub(crate) fn persist(
        tx: &WriteTransaction,
        draft: NoteDraftMemory,
    ) -> Result<PersistedNoteDraft, redb::Error> {
        let record = StoredDraftRecord {
            data: draft.into_data(),
            revision: 1,
        };
        DRAFT_STORE.put(tx, record.data.id().as_key(), &record)?;
        Ok(PersistedNoteDraft::from_record(record))
    }

    pub(crate) fn get(
        tx: &ReadTransaction,
        id: DraftId,
    ) -> Result<Option<PersistedNoteDraft>, redb::Error> {
        Ok(DRAFT_STORE
            .reader(tx)?
            .get(id.as_key())?
            .map(PersistedNoteDraft::from_record))
    }

    pub(crate) fn get_in_write(
        tx: &WriteTransaction,
        id: DraftId,
    ) -> Result<Option<PersistedNoteDraft>, redb::Error> {
        Ok(DRAFT_STORE
            .get_in_write(tx, id.as_key())?
            .map(PersistedNoteDraft::from_record))
    }

    pub(crate) fn list(tx: &ReadTransaction) -> Result<Vec<PersistedNoteDraft>, redb::Error> {
        let reader = DRAFT_STORE.reader(tx)?;
        let mut drafts = reader
            .iter()?
            .map(|item| item.map(|(_, record)| PersistedNoteDraft::from_record(record)))
            .collect::<Result<Vec<_>, _>>()?;
        drafts.sort_by(|a, b| b.data.updated_at_ms().cmp(&a.data.updated_at_ms()));
        Ok(drafts)
    }

    pub(crate) fn update(
        tx: &WriteTransaction,
        mut draft: PersistedNoteDraft,
        expected_revision: Option<u64>,
    ) -> Result<PersistedNoteDraft, DraftStoreError> {
        if expected_revision.is_some_and(|expected| expected != draft.revision()) {
            return Err(DraftStoreError::RevisionConflict {
                expected: expected_revision.unwrap_or_default(),
                actual: draft.revision(),
            });
        }
        draft.state.revision = draft.state.revision.saturating_add(1);
        let record = StoredDraftRecord {
            data: draft.data.clone(),
            revision: draft.revision(),
        };
        DRAFT_STORE.put(tx, record.data.id().as_key(), &record)?;
        Ok(draft)
    }

    pub(crate) fn delete(tx: &WriteTransaction, id: DraftId) -> Result<bool, redb::Error> {
        DRAFT_STORE.delete(tx, id.as_key())
    }

    pub(crate) fn receipt_in_write(
        tx: &WriteTransaction,
        id: DraftId,
    ) -> Result<Option<DraftCommitReceipt>, redb::Error> {
        DRAFT_COMMIT_RECEIPTS.get_in_write(tx, id.as_key())
    }

    pub(crate) fn receipt(
        tx: &ReadTransaction,
        id: DraftId,
    ) -> Result<Option<DraftCommitReceipt>, redb::Error> {
        DRAFT_COMMIT_RECEIPTS.reader(tx)?.get(id.as_key())
    }

    /// Drop the persisted draft (if any) and store a commit receipt under the same draft id.
    pub(crate) fn finish_commit(
        tx: &WriteTransaction,
        id: DraftId,
        note_id: Uuid,
        edited_from: Option<Uuid>,
        persisted: bool,
    ) -> Result<(), redb::Error> {
        if persisted {
            DRAFT_STORE.delete(tx, id.as_key())?;
        }
        DRAFT_COMMIT_RECEIPTS.put(
            tx,
            id.as_key(),
            &DraftCommitReceipt {
                note_id,
                edited_from,
            },
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DraftStoreError {
    #[error(transparent)]
    Database(#[from] redb::Error),
    #[error("draft revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
}

#[derive(Debug, Default)]
pub(crate) struct NoteDraftMemoryStore {
    drafts: HashMap<DraftId, NoteDraftMemory>,
    _not_persisted: PhantomData<Memory>,
}

impl NoteDraftMemoryStore {
    pub(crate) fn insert(&mut self, draft: NoteDraftMemory) {
        self.drafts.insert(draft.data().id(), draft);
    }
    pub(crate) fn get(&self, id: DraftId) -> Option<&NoteDraftMemory> {
        self.drafts.get(&id)
    }
    pub(crate) fn take(&mut self, id: DraftId) -> Option<NoteDraftMemory> {
        self.drafts.remove(&id)
    }
    pub(crate) fn list(&self) -> Vec<NoteDraftMemory> {
        let mut drafts = self.drafts.values().cloned().collect::<Vec<_>>();
        drafts.sort_by(|a, b| b.data().updated_at_ms().cmp(&a.data().updated_at_ms()));
        drafts
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_draft_uses_type_state_and_validated_color() {
        let mut draft = NoteDraftMemory::new();
        draft.data_mut().set_content("hello".into());
        draft.data_mut().set_color(Some("#ff0000")).unwrap();
        assert_eq!(draft.data().content(), "hello");
        assert_eq!(draft.data().color().unwrap().to_css_hex(), "#ff0000");
        assert!(draft.data_mut().set_color(Some("bad")).is_err());
    }
}
