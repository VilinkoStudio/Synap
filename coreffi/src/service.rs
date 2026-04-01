//! FFI-compatible service wrapper for Synap using UniFFI.

use std::sync::Arc;

use crate::error::FfiError;
use crate::types::{BuildInfo, NoteDTO};
use synap_core::dto::NoteDTO as CoreNoteDTO;
use synap_core::service::SynapService as CoreSynapService;

/// FFI-compatible Synap service wrapper.
pub struct SynapService {
    inner: CoreSynapService,
}

impl SynapService {
    pub fn new(inner: CoreSynapService) -> Self {
        Self { inner }
    }

    fn map_note(note: CoreNoteDTO) -> NoteDTO {
        note.into()
    }

    fn map_notes(notes: Vec<CoreNoteDTO>) -> Vec<NoteDTO> {
        notes.into_iter().map(Into::into).collect()
    }

    pub fn get_note(&self, id_or_short_id: String) -> Result<NoteDTO, FfiError> {
        self.inner
            .get_note(&id_or_short_id)
            .map(Self::map_note)
            .map_err(Into::into)
    }

    pub fn get_replies(
        &self,
        parent_id: String,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_replies(&parent_id, cursor, limit as usize)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_recent_note(
        &self,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_recent_note(cursor.as_deref(), limit.map(|value| value as usize))
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_origins(&self, child_id: String) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_origins(&child_id)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_previous_versions(&self, note_id: String) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_previous_versions(&note_id)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_next_versions(&self, note_id: String) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_next_versions(&note_id)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_other_versions(&self, note_id: String) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_other_versions(&note_id)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_deleted_notes(
        &self,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_deleted_notes(cursor.as_deref(), limit.map(|value| value as usize))
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn search(&self, query: String, limit: u32) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .search(&query, limit as usize)
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn search_tags(&self, query: String, limit: u32) -> Result<Vec<String>, FfiError> {
        self.inner
            .search_tags(&query, limit as usize)
            .map_err(Into::into)
    }

    pub fn create_note(&self, content: String, tags: Vec<String>) -> Result<NoteDTO, FfiError> {
        self.inner
            .create_note(content, tags)
            .map(Self::map_note)
            .map_err(Into::into)
    }

    pub fn reply_note(
        &self,
        parent_id: String,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, FfiError> {
        self.inner
            .reply_note(&parent_id, content, tags)
            .map(Self::map_note)
            .map_err(Into::into)
    }

    pub fn edit_note(
        &self,
        target_id: String,
        new_content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, FfiError> {
        self.inner
            .edit_note(&target_id, new_content, tags)
            .map(Self::map_note)
            .map_err(Into::into)
    }

    pub fn delete_note(&self, target_id: String) -> Result<(), FfiError> {
        self.inner.delete_note(&target_id).map_err(Into::into)
    }

    pub fn restore_note(&self, target_id: String) -> Result<(), FfiError> {
        self.inner.restore_note(&target_id).map_err(Into::into)
    }
}

/// Open a file-based database.
pub fn open(path: String) -> Result<Arc<SynapService>, FfiError> {
    let service = CoreSynapService::new(Some(path))?;
    Ok(Arc::new(SynapService::new(service)))
}

/// Open an in-memory database.
pub fn open_memory() -> Result<Arc<SynapService>, FfiError> {
    let service = CoreSynapService::new(None)?;
    Ok(Arc::new(SynapService::new(service)))
}

pub fn get_build_info() -> BuildInfo {
    synap_core::build_info().into()
}

pub fn get_version_string() -> String {
    synap_core::version_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = open_memory().unwrap();
        let notes = service.get_recent_note(None, None).unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn test_create_and_get_note() {
        let service = open_memory().unwrap();
        let note = service
            .create_note("My first note".to_string(), vec!["rust".to_string()])
            .unwrap();

        let retrieved = service.get_note(note.id.clone()).unwrap();
        assert_eq!(retrieved.id, note.id);
        assert_eq!(retrieved.content, "My first note");
        assert_eq!(retrieved.tags, vec!["rust"]);
    }

    #[test]
    fn test_edit_note_returns_new_version() {
        let service = open_memory().unwrap();
        let original = service
            .create_note("Original".to_string(), vec!["draft".to_string()])
            .unwrap();
        let edited = service
            .edit_note(
                original.id.clone(),
                "Edited".to_string(),
                vec!["published".to_string()],
            )
            .unwrap();

        assert_ne!(original.id, edited.id);
        assert_eq!(edited.content, "Edited");
        assert_eq!(edited.tags, vec!["published"]);
    }

    #[test]
    fn test_get_origins_returns_only_parent_layer() {
        let service = open_memory().unwrap();
        let root = service.create_note("Root".to_string(), vec![]).unwrap();
        let middle = service
            .reply_note(root.id.clone(), "Middle".to_string(), vec![])
            .unwrap();
        let leaf = service
            .reply_note(middle.id.clone(), "Leaf".to_string(), vec![])
            .unwrap();

        let v2 = service
            .edit_note(
                root.id.clone(),
                "Root v2".to_string(),
                vec!["published".to_string()],
            )
            .unwrap();

        let origins = service.get_origins(leaf.id).unwrap();
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0].id, middle.id);

        let previous = service.get_previous_versions(v2.id.clone()).unwrap();
        assert_eq!(previous.len(), 1);
        assert_eq!(previous[0].id, root.id);

        let next = service.get_next_versions(root.id.clone()).unwrap();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, v2.id);

        let other_versions = service.get_other_versions(v2.id).unwrap();
        assert_eq!(other_versions.len(), 1);
        assert_eq!(other_versions[0].id, root.id);
    }

    #[test]
    fn test_deleted_notes_and_restore_are_exposed() {
        let service = open_memory().unwrap();
        let first = service.create_note("First".to_string(), vec![]).unwrap();
        let second = service.create_note("Second".to_string(), vec![]).unwrap();

        service.delete_note(first.id.clone()).unwrap();
        service.delete_note(second.id.clone()).unwrap();

        let deleted = service.get_deleted_notes(None, Some(10)).unwrap();
        assert_eq!(deleted.len(), 2);
        assert_eq!(deleted[0].id, second.id);
        assert_eq!(deleted[1].id, first.id);

        assert!(matches!(
            service.get_note(second.id.clone()),
            Err(FfiError::NotFound)
        ));

        service.restore_note(second.id.clone()).unwrap();

        let restored = service.get_note(second.id).unwrap();
        assert_eq!(restored.content, "Second");
    }

    #[test]
    fn test_version_info_is_exposed() {
        let info = get_build_info();
        assert!(!info.crate_version.is_empty());
        assert!(!info.git_branch.is_empty());
        assert!(!info.git_commit.is_empty());
        assert!(!info.git_short_commit.is_empty());
        assert!(!info.display_version.is_empty());

        let version = get_version_string();
        assert_eq!(version, info.display_version);
    }
}
