use crate::{
    core::{CoreResult, DesktopCore},
    domain::{HomeData, NoteDetailData},
};
use synap_core::dto::NoteDTO;

const PAGE_SIZE: usize = 50;

pub fn load_home(core: &dyn DesktopCore, query: &str) -> CoreResult<HomeData> {
    let trimmed = query.trim();

    let notes = if trimmed.is_empty() {
        let page = core.recent_notes_page(None, Some(PAGE_SIZE))?;
        let has_more = page.next_cursor.is_some();
        HomeData {
            notes: page.notes,
            deleted_notes: Vec::new(),
            notes_cursor: page.next_cursor,
            deleted_notes_cursor: None,
            has_more_notes: has_more,
            has_more_deleted_notes: false,
        }
    } else {
        let notes = core.search(trimmed, PAGE_SIZE)?;
        HomeData {
            notes,
            deleted_notes: Vec::new(),
            notes_cursor: None,
            deleted_notes_cursor: None,
            has_more_notes: false,
            has_more_deleted_notes: false,
        }
    };

    let deleted_page = core.deleted_notes_page(None, Some(PAGE_SIZE))?;
    let has_more_deleted = deleted_page.next_cursor.is_some();

    Ok(HomeData {
        notes: notes.notes,
        deleted_notes: deleted_page.notes,
        notes_cursor: notes.notes_cursor,
        deleted_notes_cursor: deleted_page.next_cursor,
        has_more_notes: notes.has_more_notes,
        has_more_deleted_notes: has_more_deleted,
    })
}

pub fn load_more_notes(
    core: &dyn DesktopCore,
    cursor: &str,
) -> CoreResult<(Vec<NoteDTO>, Option<String>, bool)> {
    let page = core.recent_notes_page(Some(cursor), Some(PAGE_SIZE))?;
    let has_more = page.next_cursor.is_some();
    let cursor = page.next_cursor;
    Ok((page.notes, cursor, has_more))
}

pub fn load_more_deleted_notes(
    core: &dyn DesktopCore,
    cursor: &str,
) -> CoreResult<(Vec<NoteDTO>, Option<String>, bool)> {
    let page = core.deleted_notes_page(Some(cursor), Some(PAGE_SIZE))?;
    let has_more = page.next_cursor.is_some();
    let cursor = page.next_cursor;
    Ok((page.notes, cursor, has_more))
}

pub fn load_note_detail(core: &dyn DesktopCore, note_id: &str) -> CoreResult<NoteDetailData> {
    let note = core.get_note(note_id)?;
    let replies = core.replies(note_id, None, 20)?;
    let origins = core.origins(note_id)?;
    let other_versions = core.other_versions(note_id)?;

    Ok(NoteDetailData {
        note,
        replies,
        origins,
        other_versions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use synap_core::dto::{NoteVersionDTO, TimelineNotesPageDTO};

    fn make_note(id: &str, content: &str, tags: Vec<&str>, deleted: bool) -> NoteDTO {
        NoteDTO {
            id: id.to_string(),
            content: content.to_string(),
            tags: tags.into_iter().map(String::from).collect(),
            created_at: 1700000000000,
            deleted,
            reply_to: None,
            edited_from: None,
            timeline_group: None,
        }
    }

    /// A minimal mock of DesktopCore for testing usecases.
    struct MockCore {
        notes: Vec<NoteDTO>,
        deleted_notes: Vec<NoteDTO>,
    }

    impl MockCore {
        fn new() -> Self {
            Self {
                notes: vec![
                    make_note("1", "first note", vec!["rust"], false),
                    make_note("2", "second note", vec!["idea"], false),
                ],
                deleted_notes: vec![make_note("d1", "deleted note", vec![], true)],
            }
        }
    }

    impl DesktopCore for MockCore {
        fn recent_notes_page(
            &self,
            _cursor: Option<&str>,
            limit: Option<usize>,
        ) -> CoreResult<TimelineNotesPageDTO> {
            let n = limit.unwrap_or(50);
            Ok(TimelineNotesPageDTO {
                notes: self.notes.iter().take(n).cloned().collect(),
                next_cursor: None,
            })
        }

        fn deleted_notes_page(
            &self,
            _cursor: Option<&str>,
            limit: Option<usize>,
        ) -> CoreResult<TimelineNotesPageDTO> {
            let n = limit.unwrap_or(50);
            Ok(TimelineNotesPageDTO {
                notes: self.deleted_notes.iter().take(n).cloned().collect(),
                next_cursor: None,
            })
        }

        fn search(&self, query: &str, _limit: usize) -> CoreResult<Vec<NoteDTO>> {
            Ok(self
                .notes
                .iter()
                .filter(|n| n.content.contains(query))
                .cloned()
                .collect())
        }

        fn get_note(&self, id: &str) -> CoreResult<NoteDTO> {
            self.notes
                .iter()
                .chain(self.deleted_notes.iter())
                .find(|n| n.id == id)
                .cloned()
                .ok_or_else(|| synap_core::error::ServiceError::Other(anyhow::anyhow!("not found")))
        }

        fn replies(
            &self,
            _parent_id: &str,
            _cursor: Option<String>,
            _limit: usize,
        ) -> CoreResult<Vec<NoteDTO>> {
            Ok(vec![])
        }

        fn origins(&self, _note_id: &str) -> CoreResult<Vec<NoteDTO>> {
            Ok(vec![])
        }

        fn other_versions(&self, _note_id: &str) -> CoreResult<Vec<NoteVersionDTO>> {
            Ok(vec![])
        }

        fn create_note(&self, _content: String, _tags: Vec<String>) -> CoreResult<NoteDTO> {
            Ok(make_note("new", "created", vec![], false))
        }

        fn reply_note(
            &self,
            _parent_id: &str,
            _content: String,
            _tags: Vec<String>,
        ) -> CoreResult<NoteDTO> {
            Ok(make_note("reply", "replied", vec![], false))
        }

        fn edit_note(
            &self,
            _note_id: &str,
            _content: String,
            _tags: Vec<String>,
        ) -> CoreResult<NoteDTO> {
            Ok(make_note("edited", "edited", vec![], false))
        }

        fn delete_note(&self, _note_id: &str) -> CoreResult<()> {
            Ok(())
        }

        fn restore_note(&self, _note_id: &str) -> CoreResult<()> {
            Ok(())
        }

        fn search_tags(&self, _query: &str, _limit: usize) -> CoreResult<Vec<String>> {
            Ok(vec![])
        }

        fn recommend_tags(&self, _content: &str, _limit: usize) -> CoreResult<Vec<String>> {
            Ok(vec![])
        }

        fn get_all_tags(&self) -> CoreResult<Vec<String>> {
            Ok(vec!["rust".to_string(), "idea".to_string()])
        }

        fn get_notes_by_tag(&self, _tag: &str, _limit: usize) -> CoreResult<Vec<NoteDTO>> {
            Ok(vec![])
        }

        fn get_recent_sessions(
            &self,
            _cursor: Option<&str>,
            _limit: Option<usize>,
        ) -> CoreResult<synap_core::dto::TimelineSessionsPageDTO> {
            Ok(synap_core::dto::TimelineSessionsPageDTO {
                sessions: vec![],
                next_cursor: None,
            })
        }

        fn get_local_identity(&self) -> CoreResult<synap_core::dto::LocalIdentityDTO> {
            Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                "not implemented"
            )))
        }

        fn get_peers(&self) -> CoreResult<Vec<synap_core::dto::PeerDTO>> {
            Ok(vec![])
        }

        fn trust_peer(
            &self,
            _public_key: &[u8],
            _note: Option<String>,
        ) -> CoreResult<synap_core::dto::PeerDTO> {
            Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                "not implemented"
            )))
        }

        fn update_peer_note(
            &self,
            _peer_id: &str,
            _note: Option<String>,
        ) -> CoreResult<synap_core::dto::PeerDTO> {
            Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                "not implemented"
            )))
        }

        fn delete_peer(&self, _peer_id: &str) -> CoreResult<()> {
            Ok(())
        }

        fn get_recent_sync_sessions(
            &self,
            _limit: Option<usize>,
        ) -> CoreResult<Vec<synap_core::dto::SyncSessionRecordDTO>> {
            Ok(vec![])
        }

        fn ensure_sync_listener_started(
            &self,
            _preferred_port: u16,
        ) -> CoreResult<corenet::ListenerState> {
            Ok(corenet::ListenerState::default())
        }

        fn discovered_sync_peers(&self) -> Vec<corenet::DiscoveredPeer> {
            vec![]
        }

        fn connect_and_sync(
            &self,
            _host: &str,
            _port: u16,
        ) -> CoreResult<synap_core::dto::SyncSessionDTO> {
            Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                "not implemented"
            )))
        }

        fn sync_connections(&self) -> Vec<crate::domain::SyncConnectionRecord> {
            vec![]
        }

        fn save_sync_connection(
            &self,
            _host: &str,
            _port: u16,
        ) -> CoreResult<crate::domain::SyncConnectionRecord> {
            Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                "not implemented"
            )))
        }

        fn delete_sync_connection(&self, _connection_id: &str) -> CoreResult<()> {
            Ok(())
        }
    }

    #[test]
    fn load_home_empty_query_returns_recent_and_deleted() {
        let core = MockCore::new();
        let home = load_home(&core, "").unwrap();
        assert_eq!(home.notes.len(), 2);
        assert_eq!(home.deleted_notes.len(), 1);
        assert!(!home.has_more_notes);
        assert!(!home.has_more_deleted_notes);
    }

    #[test]
    fn load_home_search_query_returns_filtered() {
        let core = MockCore::new();
        let home = load_home(&core, "first").unwrap();
        assert_eq!(home.notes.len(), 1);
        assert_eq!(home.notes[0].id, "1");
    }

    #[test]
    fn load_home_search_with_whitespace() {
        let core = MockCore::new();
        let home = load_home(&core, "  first  ").unwrap();
        assert_eq!(home.notes.len(), 1);
    }

    #[test]
    fn load_more_notes_returns_page() {
        let core = MockCore::new();
        let (notes, cursor, has_more) = load_more_notes(&core, "cursor").unwrap();
        assert_eq!(notes.len(), 2);
        assert!(cursor.is_none());
        assert!(!has_more);
    }

    #[test]
    fn load_more_deleted_notes_returns_page() {
        let core = MockCore::new();
        let (notes, cursor, has_more) = load_more_deleted_notes(&core, "cursor").unwrap();
        assert_eq!(notes.len(), 1);
        assert!(cursor.is_none());
        assert!(!has_more);
    }

    #[test]
    fn load_note_detail_aggregates_data() {
        let core = MockCore::new();
        let detail = load_note_detail(&core, "1").unwrap();
        assert_eq!(detail.note.id, "1");
        assert!(detail.replies.is_empty());
        assert!(detail.origins.is_empty());
        assert!(detail.other_versions.is_empty());
    }

    #[test]
    fn load_note_detail_not_found() {
        let core = MockCore::new();
        let result = load_note_detail(&core, "nonexistent");
        assert!(result.is_err());
    }
}

