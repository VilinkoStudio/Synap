//! Stable append-only note command base shared by classic APIs and draft commit.
//!
//! Pipeline: build a [`NoteSnapshot`] (content only) → wrap as [`AppendNoteCommand`]
//! (content + optional edit/reply edges) → [`SynapService::append_note_in_tx`] writes
//! the note block / DAG edges → [`SynapService::finish_note_append`] updates derived indexes.
//! Classic APIs (`create_note` / `edit_note` / …) and `draft_commit` both end here.

use super::*;
use crate::models::tag_metadata::{split_storage_tags, TagMetadata};

/// Frozen note body ready for ledger append. No relationship edges.
///
/// `storage_tags` are already merged (display tags + `$color(...)` / unknown `$kind(...)`).
/// Edges (`edit_from` / `reply_to`) live on [`AppendNoteCommand`], not here.
#[derive(Debug, Clone)]
pub(crate) struct NoteSnapshot {
    pub(crate) content: String,
    pub(crate) storage_tags: Vec<String>,
}

/// One append-only ledger write: a snapshot plus 0–2 relationship edges.
///
/// Variants are the product of optional `base` (NOTE_EDIT) and optional `parent`
/// (NOTE_LINK). Prefer thinking in those two options when extending this type.
#[derive(Debug, Clone)]
pub(crate) enum AppendNoteCommand {
    Create {
        snapshot: NoteSnapshot,
    },
    /// New version of `base` (`NOTE_EDIT: base → new`).
    Edit {
        base: Uuid,
        snapshot: NoteSnapshot,
    },
    /// Child of `parent` (`NOTE_LINK: parent → new`).
    Reply {
        parent: Uuid,
        snapshot: NoteSnapshot,
    },
    /// Both edges: edit lineage and reply parent.
    EditReply {
        base: Uuid,
        parent: Uuid,
        snapshot: NoteSnapshot,
    },
}

impl AppendNoteCommand {
    pub(crate) fn edited_from(&self) -> Option<Uuid> {
        match self {
            Self::Edit { base, .. } | Self::EditReply { base, .. } => Some(*base),
            Self::Create { .. } | Self::Reply { .. } => None,
        }
    }

    fn snapshot(self) -> NoteSnapshot {
        match self {
            Self::Create { snapshot }
            | Self::Edit { snapshot, .. }
            | Self::Reply { snapshot, .. }
            | Self::EditReply { snapshot, .. } => snapshot,
        }
    }
}

impl SynapService {
    /// Shared write kernel. Validates live targets, materializes tags, creates or
    /// edits a note block, then optionally attaches a reply edge. No index side effects.
    pub(crate) fn append_note_in_tx(
        &self,
        tx: &WriteTransaction,
        command: AppendNoteCommand,
    ) -> Result<Note, ServiceError> {
        let edited_from = command.edited_from();
        let reply_to = match &command {
            AppendNoteCommand::Reply { parent, .. }
            | AppendNoteCommand::EditReply { parent, .. } => Some(*parent),
            _ => None,
        };

        if let Some(base) = edited_from {
            if !Note::is_live_in_write(tx, &base)? {
                return Err(ServiceError::NotFound(base.to_string()));
            }
        }
        if let Some(parent) = reply_to {
            if !Note::is_live_in_write(tx, &parent)? {
                return Err(ServiceError::NotFound(parent.to_string()));
            }
        }

        let snapshot = command.snapshot();
        let tags = self.materialize_tags(tx, snapshot.storage_tags)?;
        let note = match edited_from {
            Some(base) => NoteRef::new(base, false).edit(tx, snapshot.content, tags)?,
            None => Note::create(tx, snapshot.content, tags)?,
        };

        if let Some(parent) = reply_to {
            NoteRef::new(parent, false).reply_to(tx, &note.get_id())?;
        }
        Ok(note)
    }

    /// Post-commit derived-state updates. Create reserves embedding / tag indexes;
    /// edit refreshes search indexes.
    pub(crate) fn finish_note_append(
        &self,
        note: Note,
        edited_from: Option<Uuid>,
    ) -> Result<NoteDTO, ServiceError> {
        if let Some(_previous) = edited_from {
            self.refresh_search_indexes()?;
        } else {
            self.note_searcher.insert(note.clone());
            self.reserve_note_embedding(&note)?;
            self.refresh_tag_indexes()?;
        }
        self.with_read(|_tx, reader| self.note_to_dto(note, reader))
    }

    /// Normalize display tags + merge metadata into a ledger-ready snapshot.
    /// Caller must still reject empty content where required.
    fn snapshot_from_display(
        content: String,
        tags: Vec<String>,
        metadata: &TagMetadata,
    ) -> NoteSnapshot {
        let display = Self::normalize_display_tag_inputs(tags);
        NoteSnapshot {
            content: content.trim().to_owned(),
            storage_tags: Self::encode_storage_tags(display, metadata),
        }
    }

    /// Convenience path for classic APIs: one write tx, then finish indexes (no draft/receipt).
    fn append_classic(&self, command: AppendNoteCommand) -> Result<NoteDTO, ServiceError> {
        let edited_from = command.edited_from();
        let note = self.with_write(|tx| self.append_note_in_tx(tx, command))?;
        self.finish_note_append(note, edited_from)
    }

    pub fn create_note(&self, content: String, tags: Vec<String>) -> Result<NoteDTO, ServiceError> {
        let snapshot = Self::snapshot_from_display(content, tags, &TagMetadata::default());
        if snapshot.content.is_empty() {
            return Err(ServiceError::InvalidDraft(
                "note content cannot be empty".into(),
            ));
        }
        self.append_classic(AppendNoteCommand::Create { snapshot })
    }

    pub fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let parent = Self::parse_id(parent_id)?;
        let snapshot = Self::snapshot_from_display(content, tags, &TagMetadata::default());
        if snapshot.content.is_empty() {
            return Err(ServiceError::InvalidDraft(
                "note content cannot be empty".into(),
            ));
        }
        self.append_classic(AppendNoteCommand::Reply { parent, snapshot })
    }

    pub fn edit_note(
        &self,
        target_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let base = Self::parse_id(target_id)?;
        let metadata = self.with_read(|_tx, reader| {
            let note_ref = Self::require_live_note_ref(reader, base, target_id)?;
            let note = note_ref
                .hydrate(reader)?
                .ok_or_else(|| ServiceError::NotFound(target_id.to_owned()))?;
            Ok(split_storage_tags(Self::note_storage_tag_strings(&note, reader)?).metadata)
        })?;
        let snapshot = Self::snapshot_from_display(content, tags, &metadata);
        if snapshot.content.is_empty() {
            return Err(ServiceError::InvalidDraft(
                "note content cannot be empty".into(),
            ));
        }
        self.append_classic(AppendNoteCommand::Edit { base, snapshot })
    }

    pub fn set_note_color(
        &self,
        target_id: &str,
        color: Option<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let base = Self::parse_id(target_id)?;
        let (content, display_tags, mut metadata) = self.with_read(|_tx, reader| {
            let note_ref = Self::require_live_note_ref(reader, base, target_id)?;
            let note = note_ref
                .hydrate(reader)?
                .ok_or_else(|| ServiceError::NotFound(target_id.to_owned()))?;
            let split = split_storage_tags(Self::note_storage_tag_strings(&note, reader)?);
            Ok((
                note.content().to_owned(),
                split.display_tags,
                split.metadata,
            ))
        })?;
        metadata.color = crate::models::tag_metadata::parse_color_input(color.as_deref())?;
        let snapshot = Self::snapshot_from_display(content, display_tags, &metadata);
        self.append_classic(AppendNoteCommand::Edit { base, snapshot })
    }

    pub fn delete_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.del(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        self.delete_note_embedding(uuid)?;
        Ok(())
    }

    pub fn restore_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.restore(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }
}
