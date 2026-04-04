//! Application state for the desktop client.

use crate::core::{Result, ServiceWrapper};
use synap_core::dto::NoteDTO;

#[derive(Debug, Default)]
pub struct AppState {
    pub notes: Vec<NoteDTO>,
    pub deleted_notes: Vec<NoteDTO>,
    pub selected_note: Option<NoteDTO>,
    pub selected_replies: Vec<NoteDTO>,
    pub selected_origins: Vec<NoteDTO>,
    pub selected_versions: Vec<NoteDTO>,
    pub compose_content: String,
    pub compose_tags: String,
    pub detail_content: String,
    pub detail_tags: String,
    pub search_query: String,
    pub status: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_note_id(&self) -> Option<&str> {
        self.selected_note.as_ref().map(|note| note.id.as_str())
    }

    pub fn clear_selection(&mut self) {
        self.selected_note = None;
        self.selected_replies.clear();
        self.selected_origins.clear();
        self.selected_versions.clear();
        self.detail_content.clear();
        self.detail_tags.clear();
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
    }

    pub fn refresh(&mut self) -> Result<()> {
        let query = self.search_query.trim();
        self.notes = if query.is_empty() {
            ServiceWrapper::recent_notes(None, Some(50))?
        } else {
            ServiceWrapper::search(query, 50)?
        };
        self.deleted_notes = ServiceWrapper::deleted_notes(None, Some(50))?;

        if let Some(id) = self.selected_note_id().map(str::to_owned) {
            if self.reload_selected(&id).is_err() {
                self.clear_selection();
            }
        } else if let Some(first) = self.notes.first() {
            self.reload_selected(&first.id.clone())?;
        }

        Ok(())
    }

    pub fn select_note(&mut self, id: &str) -> Result<()> {
        self.reload_selected(id)
    }

    fn reload_selected(&mut self, id: &str) -> Result<()> {
        let note = ServiceWrapper::get_note(id)?;
        self.detail_content = note.content.clone();
        self.detail_tags = note.tags.join(", ");
        self.selected_replies = ServiceWrapper::replies(id, None, 20)?;
        self.selected_origins = ServiceWrapper::origins(id)?;
        self.selected_versions = ServiceWrapper::other_versions(id)?;
        self.selected_note = Some(note);
        Ok(())
    }
}
