//! Core bridge layer for desktop application.

use once_cell::sync::OnceCell;
use std::{io, sync::Mutex};

use synap_core::{
    dto::{NoteDTO, NoteVersionDTO},
    error::ServiceError,
    service::SynapService,
};

pub type Result<T> = std::result::Result<T, ServiceError>;

static SERVICE: OnceCell<Mutex<SynapService>> = OnceCell::new();

#[derive(Debug)]
pub struct ServiceWrapper;

impl ServiceWrapper {
    pub fn init() -> Result<()> {
        if SERVICE.get().is_some() {
            return Ok(());
        }

        let db_path =
            std::env::var("SYNAP_DESKTOP_DB").unwrap_or_else(|_| "synap-desktop.redb".to_string());
        let service = SynapService::new(Some(db_path))?;

        SERVICE.set(Mutex::new(service)).map_err(|_| {
            ServiceError::Other(io::Error::other("failed to initialize service").into())
        })?;

        Ok(())
    }

    fn with_service<T>(f: impl FnOnce(&SynapService) -> Result<T>) -> Result<T> {
        let service = SERVICE.get().ok_or_else(|| {
            ServiceError::Other(io::Error::other("service has not been initialized").into())
        })?;
        let guard = service
            .lock()
            .map_err(|_| ServiceError::Other(io::Error::other("service lock poisoned").into()))?;

        f(&guard)
    }

    pub fn recent_notes(cursor: Option<&str>, limit: Option<usize>) -> Result<Vec<NoteDTO>> {
        Self::with_service(|service| service.get_recent_note(cursor, limit))
    }

    pub fn deleted_notes(cursor: Option<&str>, limit: Option<usize>) -> Result<Vec<NoteDTO>> {
        Self::with_service(|service| service.get_deleted_notes(cursor, limit))
    }

    pub fn search(query: &str, limit: usize) -> Result<Vec<NoteDTO>> {
        Self::with_service(|service| service.search(query, limit))
    }

    pub fn get_note(id: &str) -> Result<NoteDTO> {
        Self::with_service(|service| service.get_note(id))
    }

    pub fn replies(parent_id: &str, cursor: Option<String>, limit: usize) -> Result<Vec<NoteDTO>> {
        Self::with_service(|service| service.get_replies(parent_id, cursor, limit))
    }

    pub fn origins(note_id: &str) -> Result<Vec<NoteDTO>> {
        Self::with_service(|service| service.get_origins(note_id))
    }

    pub fn other_versions(note_id: &str) -> Result<Vec<NoteVersionDTO>> {
        Self::with_service(|service| service.get_other_versions(note_id))
    }

    pub fn create_note(content: String, tags: Vec<String>) -> Result<NoteDTO> {
        Self::with_service(|service| service.create_note(content, tags))
    }

    pub fn reply_note(parent_id: &str, content: String, tags: Vec<String>) -> Result<NoteDTO> {
        Self::with_service(|service| service.reply_note(parent_id, content, tags))
    }

    pub fn edit_note(note_id: &str, content: String, tags: Vec<String>) -> Result<NoteDTO> {
        Self::with_service(|service| service.edit_note(note_id, content, tags))
    }

    pub fn delete_note(note_id: &str) -> Result<()> {
        Self::with_service(|service| service.delete_note(note_id))
    }

    pub fn restore_note(note_id: &str) -> Result<()> {
        Self::with_service(|service| service.restore_note(note_id))
    }
}
