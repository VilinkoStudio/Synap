use synap_core::{
    dto::{NoteDTO, TimelineNotesPageDTO, TimelineSessionsPageDTO},
    error::ServiceError,
    service::{SynapService, TimelineDirection},
};

pub type CoreResult<T> = Result<T, ServiceError>;

pub trait DesktopCore {
    fn recent_notes(&self, cursor: Option<&str>, limit: Option<usize>) -> CoreResult<Vec<NoteDTO>>;
    fn recent_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO>;
    fn deleted_notes(&self, cursor: Option<&str>, limit: Option<usize>)
        -> CoreResult<Vec<NoteDTO>>;
    fn deleted_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO>;
    fn search(&self, query: &str, limit: usize) -> CoreResult<Vec<NoteDTO>>;

    fn get_note(&self, id: &str) -> CoreResult<NoteDTO>;
    fn replies(
        &self,
        parent_id: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> CoreResult<Vec<NoteDTO>>;
    fn origins(&self, note_id: &str) -> CoreResult<Vec<NoteDTO>>;
    fn other_versions(&self, note_id: &str) -> CoreResult<Vec<NoteDTO>>;

    fn create_note(&self, content: String, tags: Vec<String>) -> CoreResult<NoteDTO>;
    fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> CoreResult<NoteDTO>;
    fn edit_note(&self, note_id: &str, content: String, tags: Vec<String>) -> CoreResult<NoteDTO>;
    fn delete_note(&self, note_id: &str) -> CoreResult<()>;
    fn restore_note(&self, note_id: &str) -> CoreResult<()>;

    fn search_tags(&self, query: &str, limit: usize) -> CoreResult<Vec<String>>;
    fn recommend_tags(&self, content: &str, limit: usize) -> CoreResult<Vec<String>>;
    fn get_all_tags(&self) -> CoreResult<Vec<String>>;
    fn get_notes_by_tag(&self, tag: &str, limit: usize) -> CoreResult<Vec<NoteDTO>>;
    fn get_recent_sessions(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineSessionsPageDTO>;
}

pub struct SynapCoreAdapter {
    service: SynapService,
}

impl SynapCoreAdapter {
    pub fn new_from_env() -> CoreResult<Self> {
        let db_path =
            std::env::var("SYNAP_DESKTOP_DB").unwrap_or_else(|_| "synap-desktop.redb".to_string());
        let service = SynapService::new(Some(db_path))?;
        Ok(Self { service })
    }
}

impl DesktopCore for SynapCoreAdapter {
    fn recent_notes(&self, cursor: Option<&str>, limit: Option<usize>) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_recent_note(cursor, limit)
    }

    fn recent_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO> {
        self.service
            .get_recent_notes_page(cursor, TimelineDirection::Older, limit)
    }

    fn deleted_notes(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_deleted_notes(cursor, limit)
    }

    fn deleted_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO> {
        self.service.get_filtered_notes_page(
            vec![],
            false,
            false,
            synap_core::service::FilteredNoteStatus::Deleted,
            cursor,
            TimelineDirection::Older,
            limit,
        )
    }

    fn search(&self, query: &str, limit: usize) -> CoreResult<Vec<NoteDTO>> {
        self.service.search(query, limit)
    }

    fn get_note(&self, id: &str) -> CoreResult<NoteDTO> {
        self.service.get_note(id)
    }

    fn replies(
        &self,
        parent_id: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_replies(parent_id, cursor, limit)
    }

    fn origins(&self, note_id: &str) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_origins(note_id)
    }

    fn other_versions(&self, note_id: &str) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_other_versions(note_id)
    }

    fn create_note(&self, content: String, tags: Vec<String>) -> CoreResult<NoteDTO> {
        self.service.create_note(content, tags)
    }

    fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> CoreResult<NoteDTO> {
        self.service.reply_note(parent_id, content, tags)
    }

    fn edit_note(&self, note_id: &str, content: String, tags: Vec<String>) -> CoreResult<NoteDTO> {
        self.service.edit_note(note_id, content, tags)
    }

    fn delete_note(&self, note_id: &str) -> CoreResult<()> {
        self.service.delete_note(note_id)
    }

    fn restore_note(&self, note_id: &str) -> CoreResult<()> {
        self.service.restore_note(note_id)
    }

    fn search_tags(&self, query: &str, limit: usize) -> CoreResult<Vec<String>> {
        self.service.search_tags(query, limit)
    }

    fn recommend_tags(&self, content: &str, limit: usize) -> CoreResult<Vec<String>> {
        self.service.recommend_tag(content, limit)
    }

    fn get_all_tags(&self) -> CoreResult<Vec<String>> {
        self.service.get_all_tags()
    }

    fn get_notes_by_tag(&self, tag: &str, limit: usize) -> CoreResult<Vec<NoteDTO>> {
        self.service.get_notes_by_tag(tag, None, Some(limit))
    }

    fn get_recent_sessions(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineSessionsPageDTO> {
        self.service.get_recent_sessions(cursor, limit)
    }
}
