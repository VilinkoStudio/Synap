use super::*;
use crate::{
    dto::NoteDraftDTO,
    models::{
        note_draft::{
            DraftData, DraftId, DraftOrigin, DraftRepository, DraftStoreError, NoteDraftMemory,
            NoteDraftMemoryStore, PersistedNoteDraft,
        },
        tag_metadata::split_storage_tags,
    },
    service::note_command::{AppendNoteCommand, NoteSnapshot},
};

impl From<crate::models::tag_metadata::TagMetadataError> for ServiceError {
    fn from(value: crate::models::tag_metadata::TagMetadataError) -> Self {
        Self::InvalidTagMetadata(value.to_string())
    }
}

impl From<DraftStoreError> for ServiceError {
    fn from(value: DraftStoreError) -> Self {
        match value {
            DraftStoreError::Database(error) => Self::Db(error),
            DraftStoreError::RevisionConflict { expected, actual } => {
                Self::DraftRevisionConflict { expected, actual }
            }
        }
    }
}

/// Service-side union over type-state drafts (memory first when resolving by id).
enum ActiveDraft {
    Memory(NoteDraftMemory),
    Persisted(PersistedNoteDraft),
}

impl ActiveDraft {
    fn data(&self) -> &DraftData {
        match self {
            Self::Memory(draft) => draft.data(),
            Self::Persisted(draft) => draft.data(),
        }
    }

    fn persisted(&self) -> bool {
        matches!(self, Self::Persisted(_))
    }

    fn revision(&self) -> u64 {
        match self {
            Self::Memory(_) => 0,
            Self::Persisted(draft) => draft.revision(),
        }
    }
}

/// Validated draft ready to append: frozen [`AppendNoteCommand`] + commit bookkeeping.
struct ReadyDraft {
    id: DraftId,
    command: AppendNoteCommand,
    persisted: bool,
}

impl SynapService {
    fn with_memory_drafts<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut NoteDraftMemoryStore) -> T,
    {
        let mut store = self.draft_store.lock().expect("draft store lock");
        f(&mut store)
    }

    fn parse_draft_id(draft_id: &str) -> Result<DraftId, ServiceError> {
        Ok(DraftId::from_uuid(Self::parse_id(draft_id)?))
    }

    fn draft_to_dto(draft: &ActiveDraft) -> NoteDraftDTO {
        let data = draft.data();
        NoteDraftDTO {
            id: data.id().as_uuid().to_string(),
            content: data.content().to_owned(),
            tags: data.tags().to_vec(),
            color: data.color().map(|color| color.to_css_hex()),
            reply_to: data.reply_to().map(|id| id.to_string()),
            edited_from: match data.origin() {
                DraftOrigin::New => None,
                DraftOrigin::Edit { base } => Some(base.to_string()),
            },
            created_at: data.created_at_ms(),
            updated_at: data.updated_at_ms(),
            persisted: draft.persisted(),
            revision: draft.revision(),
        }
    }

    fn load_active_draft(&self, id: DraftId) -> Result<Option<ActiveDraft>, ServiceError> {
        if let Some(draft) = self.with_memory_drafts(|store| store.get(id).cloned()) {
            return Ok(Some(ActiveDraft::Memory(draft)));
        }
        self.with_read(|tx, _reader| Ok(DraftRepository::get(tx, id)?.map(ActiveDraft::Persisted)))
    }

    fn require_active_draft(
        &self,
        id: DraftId,
        display_id: &str,
    ) -> Result<ActiveDraft, ServiceError> {
        self.load_active_draft(id)?
            .ok_or_else(|| ServiceError::DraftNotFound(display_id.to_owned()))
    }

    fn validate_live_target(&self, id: Uuid, display_id: &str) -> Result<(), ServiceError> {
        self.with_read(|_tx, reader| {
            Self::require_live_note_ref(reader, id, display_id)?;
            Ok(())
        })
    }

    pub fn draft_new(&self) -> Result<NoteDraftDTO, ServiceError> {
        let draft = NoteDraftMemory::new();
        let dto = Self::draft_to_dto(&ActiveDraft::Memory(draft.clone()));
        self.with_memory_drafts(|store| store.insert(draft));
        Ok(dto)
    }

    pub fn draft_from_note(&self, note_id: &str) -> Result<NoteDraftDTO, ServiceError> {
        let base = Self::parse_id(note_id)?;
        let (content, split) = self.with_read(|_tx, reader| {
            let note_ref = Self::require_live_note_ref(reader, base, note_id)?;
            let note = note_ref
                .hydrate(reader)?
                .ok_or_else(|| ServiceError::NotFound(note_id.to_owned()))?;
            let split = split_storage_tags(Self::note_storage_tag_strings(&note, reader)?);
            Ok((note.content().to_owned(), split))
        })?;

        let mut draft = NoteDraftMemory::new();
        draft.data_mut().set_content(content);
        draft.data_mut().set_tags(split.display_tags);
        draft
            .data_mut()
            .set_color(split.metadata.color.map(|c| c.to_css_hex()).as_deref())?;
        draft.data_mut().set_unknown_meta(split.metadata.unknown);
        draft.data_mut().set_origin(DraftOrigin::Edit { base });
        let dto = Self::draft_to_dto(&ActiveDraft::Memory(draft.clone()));
        self.with_memory_drafts(|store| store.insert(draft));
        Ok(dto)
    }

    pub fn draft_reply_to(&self, parent_id: &str) -> Result<NoteDraftDTO, ServiceError> {
        let parent = Self::parse_id(parent_id)?;
        self.validate_live_target(parent, parent_id)?;
        let mut draft = NoteDraftMemory::new();
        draft.data_mut().set_reply_to(Some(parent));
        let dto = Self::draft_to_dto(&ActiveDraft::Memory(draft.clone()));
        self.with_memory_drafts(|store| store.insert(draft));
        Ok(dto)
    }

    pub fn draft_get(&self, draft_id: &str) -> Result<NoteDraftDTO, ServiceError> {
        let id = Self::parse_draft_id(draft_id)?;
        Ok(Self::draft_to_dto(
            &self.require_active_draft(id, draft_id)?,
        ))
    }

    pub fn draft_list(&self) -> Result<Vec<NoteDraftDTO>, ServiceError> {
        let memory = self.with_memory_drafts(|store| store.list());
        let persisted =
            self.with_read(|tx, _reader| DraftRepository::list(tx).map_err(Into::into))?;
        let mut drafts = memory
            .into_iter()
            .map(ActiveDraft::Memory)
            .chain(persisted.into_iter().map(ActiveDraft::Persisted))
            .map(|draft| Self::draft_to_dto(&draft))
            .collect::<Vec<_>>();
        drafts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(drafts)
    }

    pub fn draft_persist(&self, draft_id: &str) -> Result<NoteDraftDTO, ServiceError> {
        let id = Self::parse_draft_id(draft_id)?;
        if let Some(existing) =
            self.with_read(|tx, _reader| DraftRepository::get(tx, id).map_err(Into::into))?
        {
            return Ok(Self::draft_to_dto(&ActiveDraft::Persisted(existing)));
        }

        let draft = self
            .with_memory_drafts(|store| store.take(id))
            .ok_or_else(|| ServiceError::DraftNotFound(draft_id.to_owned()))?;
        match self.with_write(|tx| DraftRepository::persist(tx, draft.clone()).map_err(Into::into))
        {
            Ok(persisted) => Ok(Self::draft_to_dto(&ActiveDraft::Persisted(persisted))),
            Err(error) => {
                self.with_memory_drafts(|store| store.insert(draft));
                Err(error)
            }
        }
    }

    pub fn draft_discard(&self, draft_id: &str) -> Result<(), ServiceError> {
        let id = Self::parse_draft_id(draft_id)?;
        if self.with_memory_drafts(|store| store.take(id)).is_some() {
            return Ok(());
        }
        let removed = self.with_write(|tx| DraftRepository::delete(tx, id).map_err(Into::into))?;
        if removed {
            Ok(())
        } else {
            Err(ServiceError::DraftNotFound(draft_id.to_owned()))
        }
    }

    pub fn draft_update(
        &self,
        draft_id: &str,
        content: Option<String>,
        tags: Option<Vec<String>>,
        color: Option<Option<String>>,
        reply_to: Option<Option<String>>,
        edited_from: Option<Option<String>>,
    ) -> Result<NoteDraftDTO, ServiceError> {
        self.draft_update_checked(draft_id, content, tags, color, reply_to, edited_from, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draft_update_checked(
        &self,
        draft_id: &str,
        content: Option<String>,
        tags: Option<Vec<String>>,
        color: Option<Option<String>>,
        reply_to: Option<Option<String>>,
        edited_from: Option<Option<String>>,
        expected_revision: Option<u64>,
    ) -> Result<NoteDraftDTO, ServiceError> {
        let id = Self::parse_draft_id(draft_id)?;
        let parsed_reply = reply_to
            .as_ref()
            .map(|value| value.as_deref().map(Self::parse_id).transpose())
            .transpose()?;
        let parsed_edit = edited_from
            .as_ref()
            .map(|value| value.as_deref().map(Self::parse_id).transpose())
            .transpose()?;
        if let Some(Some(target)) = parsed_reply {
            self.validate_live_target(target, &target.to_string())?;
        }
        if let Some(Some(target)) = parsed_edit {
            self.validate_live_target(target, &target.to_string())?;
        }

        let apply = |data: &mut DraftData| -> Result<(), ServiceError> {
            if let Some(value) = content.clone() {
                data.set_content(value);
            }
            if let Some(value) = tags.clone() {
                data.set_tags(Self::normalize_display_tag_inputs(value));
            }
            if let Some(value) = color.as_ref() {
                data.set_color(value.as_deref())?;
            }
            if let Some(value) = parsed_reply {
                data.set_reply_to(value);
            }
            if let Some(value) = parsed_edit {
                data.set_origin(value.map_or(DraftOrigin::New, |base| DraftOrigin::Edit { base }));
            }
            Ok(())
        };

        let memory_result = self.with_memory_drafts(|store| {
            let Some(current) = store.get(id).cloned() else {
                return None;
            };
            let mut updated = current;
            if let Err(error) = apply(updated.data_mut()) {
                return Some(Err(error));
            }
            store.insert(updated.clone());
            Some(Ok(Self::draft_to_dto(&ActiveDraft::Memory(updated))))
        });
        if let Some(result) = memory_result {
            return result;
        }

        let persisted = self.with_write(|tx| {
            let mut draft = DraftRepository::get_in_write(tx, id)?
                .ok_or_else(|| ServiceError::DraftNotFound(draft_id.to_owned()))?;
            if expected_revision.is_some_and(|revision| revision != draft.revision()) {
                return Err(ServiceError::DraftRevisionConflict {
                    expected: expected_revision.unwrap_or_default(),
                    actual: draft.revision(),
                });
            }
            apply(draft.data_mut())?;
            DraftRepository::update(tx, draft, expected_revision).map_err(Into::into)
        })?;
        Ok(Self::draft_to_dto(&ActiveDraft::Persisted(persisted)))
    }

    /// Freeze draft content into a snapshot and map origin/reply_to → command edges.
    fn ready_draft(draft: ActiveDraft) -> Result<ReadyDraft, ServiceError> {
        let persisted = draft.persisted();
        let data = match draft {
            ActiveDraft::Memory(draft) => draft.into_data(),
            ActiveDraft::Persisted(draft) => draft.into_data(),
        };
        let content = data.content().trim();
        if content.is_empty() {
            return Err(ServiceError::InvalidDraft(
                "note content cannot be empty".into(),
            ));
        }
        let display = Self::normalize_display_tag_inputs(data.tags().to_vec());
        let snapshot = NoteSnapshot {
            content: content.to_owned(),
            storage_tags: Self::encode_storage_tags(display, &data.metadata()),
        };
        let command = match (data.origin(), data.reply_to()) {
            (DraftOrigin::New, None) => AppendNoteCommand::Create { snapshot },
            (DraftOrigin::New, Some(parent)) => AppendNoteCommand::Reply { parent, snapshot },
            (DraftOrigin::Edit { base }, None) => AppendNoteCommand::Edit {
                base: *base,
                snapshot,
            },
            (DraftOrigin::Edit { base }, Some(parent)) => AppendNoteCommand::EditReply {
                base: *base,
                parent,
                snapshot,
            },
        };
        Ok(ReadyDraft {
            id: data.id(),
            command,
            persisted,
        })
    }

    fn note_from_receipt(&self, note_id: Uuid) -> Result<Note, ServiceError> {
        self.with_read(|_tx, reader| {
            reader
                .get_by_id(&note_id)?
                .ok_or_else(|| ServiceError::NotFound(note_id.to_string()))
        })
    }

    /// Append once under this draft id. Receipt makes retries idempotent.
    pub fn draft_commit(&self, draft_id: &str) -> Result<NoteDTO, ServiceError> {
        let _commit_guard = self.draft_commit_lock.lock().expect("draft commit lock");
        let id = Self::parse_draft_id(draft_id)?;
        if let Some(receipt) =
            self.with_read(|tx, _reader| DraftRepository::receipt(tx, id).map_err(Into::into))?
        {
            let note = self.note_from_receipt(receipt.note_id)?;
            return self.finish_note_append(note, receipt.edited_from);
        }

        let memory = self.with_memory_drafts(|store| store.take(id));
        let active = match memory.clone() {
            Some(draft) => ActiveDraft::Memory(draft),
            None => ActiveDraft::Persisted(
                self.with_read(|tx, _reader| DraftRepository::get(tx, id).map_err(Into::into))?
                    .ok_or_else(|| ServiceError::DraftNotFound(draft_id.to_owned()))?,
            ),
        };
        let ready = match Self::ready_draft(active) {
            Ok(ready) => ready,
            Err(error) => {
                if let Some(draft) = memory {
                    self.with_memory_drafts(|store| store.insert(draft));
                }
                return Err(error);
            }
        };
        let edited_from = ready.command.edited_from();
        let result = self.with_write(|tx| {
            if let Some(receipt) = DraftRepository::receipt_in_write(tx, ready.id)? {
                return Ok((None, receipt));
            }
            let note = self.append_note_in_tx(tx, ready.command.clone())?;
            DraftRepository::finish_commit(
                tx,
                ready.id,
                note.get_id(),
                edited_from,
                ready.persisted,
            )?;
            Ok((
                Some(note.clone()),
                crate::models::note_draft::DraftCommitReceipt {
                    note_id: note.get_id(),
                    edited_from,
                },
            ))
        });
        let (note, receipt) = match result {
            Ok(value) => value,
            Err(error) => {
                if let Some(draft) = memory {
                    self.with_memory_drafts(|store| store.insert(draft));
                }
                return Err(error);
            }
        };
        let note = match note {
            Some(note) => note,
            None => self.note_from_receipt(receipt.note_id)?,
        };
        self.finish_note_append(note, receipt.edited_from)
    }
}
