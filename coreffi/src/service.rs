//! FFI-compatible service wrapper for Synap using UniFFI.

use std::{
    io::{Read, Write},
    sync::Arc,
};

use crate::error::FfiError;
use crate::types::{
    BuildInfo, FilteredNoteStatus, LocalIdentityDTO, NoteDTO, PeerDTO, ShareStatsDTO,
    SyncSessionDTO, SyncSessionRecordDTO, TimelineDirection, TimelineNotesPageDTO,
    TimelineSessionsPageDTO,
};
use synap_core::dto::NoteDTO as CoreNoteDTO;
use synap_core::service::SynapService as CoreSynapService;

struct ForeignSyncTransport {
    inner: Box<dyn crate::SyncTransport>,
}

impl ForeignSyncTransport {
    fn new(inner: Box<dyn crate::SyncTransport>) -> Self {
        Self { inner }
    }
}

impl Read for ForeignSyncTransport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let bytes = self
            .inner
            .read(buf.len().try_into().unwrap_or(u32::MAX))
            .map_err(std::io::Error::other)?;
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        Ok(len)
    }
}

impl Write for ForeignSyncTransport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner
            .write(buf.to_vec())
            .map_err(std::io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for ForeignSyncTransport {
    fn drop(&mut self) {
        let _ = self.inner.close();
    }
}

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

    fn map_note_page(page: synap_core::dto::TimelineNotesPageDTO) -> TimelineNotesPageDTO {
        page.into()
    }

    fn map_session_page(page: synap_core::dto::TimelineSessionsPageDTO) -> TimelineSessionsPageDTO {
        page.into()
    }

    fn map_share_stats(stats: synap_core::dto::ShareStatsDTO) -> ShareStatsDTO {
        stats.into()
    }

    fn map_sync_session(session: synap_core::dto::SyncSessionDTO) -> SyncSessionDTO {
        session.into()
    }

    fn map_sync_session_records(
        records: Vec<synap_core::dto::SyncSessionRecordDTO>,
    ) -> Vec<SyncSessionRecordDTO> {
        records.into_iter().map(Into::into).collect()
    }

    fn map_local_identity(identity: synap_core::dto::LocalIdentityDTO) -> LocalIdentityDTO {
        identity.into()
    }

    fn map_peer(peer: synap_core::dto::PeerDTO) -> PeerDTO {
        peer.into()
    }

    fn map_peers(peers: Vec<synap_core::dto::PeerDTO>) -> Vec<PeerDTO> {
        peers.into_iter().map(Into::into).collect()
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

    pub fn get_recent_notes_page(
        &self,
        cursor: Option<String>,
        direction: TimelineDirection,
        limit: Option<u32>,
    ) -> Result<TimelineNotesPageDTO, FfiError> {
        self.inner
            .get_recent_notes_page(
                cursor.as_deref(),
                direction.into(),
                limit.map(|value| value as usize),
            )
            .map(Self::map_note_page)
            .map_err(Into::into)
    }

    pub fn get_recent_sessions_page(
        &self,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<TimelineSessionsPageDTO, FfiError> {
        self.inner
            .get_recent_sessions(cursor.as_deref(), limit.map(|value| value as usize))
            .map(Self::map_session_page)
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

    pub fn recommend_tag(&self, content: String, limit: u32) -> Result<Vec<String>, FfiError> {
        self.inner
            .recommend_tag(&content, limit as usize)
            .map_err(Into::into)
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>, FfiError> {
        self.inner.get_all_tags().map_err(Into::into)
    }

    pub fn get_notes_by_tag(
        &self,
        tag: String,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_notes_by_tag(&tag, cursor.as_deref(), limit.map(|value| value as usize))
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_filtered_notes(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<NoteDTO>, FfiError> {
        self.inner
            .get_filtered_notes(
                selected_tags,
                include_untagged,
                tag_filter_enabled,
                status.into(),
                cursor.as_deref(),
                limit.map(|value| value as usize),
            )
            .map(Self::map_notes)
            .map_err(Into::into)
    }

    pub fn get_filtered_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<String>,
        direction: TimelineDirection,
        limit: Option<u32>,
    ) -> Result<TimelineNotesPageDTO, FfiError> {
        self.inner
            .get_filtered_notes_page(
                selected_tags,
                include_untagged,
                tag_filter_enabled,
                status.into(),
                cursor.as_deref(),
                direction.into(),
                limit.map(|value| value as usize),
            )
            .map(Self::map_note_page)
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

    pub fn export_share(&self, note_ids: Vec<String>) -> Result<Vec<u8>, FfiError> {
        self.inner.export_share(&note_ids).map_err(Into::into)
    }

    pub fn import_share(&self, bytes: Vec<u8>) -> Result<ShareStatsDTO, FfiError> {
        self.inner
            .import_share(&bytes)
            .map(Self::map_share_stats)
            .map_err(Into::into)
    }

    pub fn get_local_identity(&self) -> Result<LocalIdentityDTO, FfiError> {
        self.inner
            .get_local_identity()
            .map(Self::map_local_identity)
            .map_err(Into::into)
    }

    pub fn get_peers(&self) -> Result<Vec<PeerDTO>, FfiError> {
        self.inner
            .get_peers()
            .map(Self::map_peers)
            .map_err(Into::into)
    }

    pub fn trust_peer(
        &self,
        public_key: Vec<u8>,
        note: Option<String>,
    ) -> Result<PeerDTO, FfiError> {
        self.inner
            .trust_peer(&public_key, note)
            .map(Self::map_peer)
            .map_err(Into::into)
    }

    pub fn update_peer_note(
        &self,
        peer_id: String,
        note: Option<String>,
    ) -> Result<PeerDTO, FfiError> {
        self.inner
            .update_peer_note(&peer_id, note)
            .map(Self::map_peer)
            .map_err(Into::into)
    }

    pub fn set_peer_status(
        &self,
        peer_id: String,
        status: crate::types::PeerTrustStatusDTO,
    ) -> Result<PeerDTO, FfiError> {
        self.inner
            .set_peer_status(&peer_id, status.into())
            .map(Self::map_peer)
            .map_err(Into::into)
    }

    pub fn delete_peer(&self, peer_id: String) -> Result<(), FfiError> {
        self.inner.delete_peer(&peer_id).map_err(Into::into)
    }

    pub fn get_recent_sync_sessions(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<SyncSessionRecordDTO>, FfiError> {
        self.inner
            .get_recent_sync_sessions(limit.map(|value| value as usize))
            .map(Self::map_sync_session_records)
            .map_err(Into::into)
    }

    pub fn initiate_sync(
        &self,
        transport: Box<dyn crate::SyncTransport>,
    ) -> Result<SyncSessionDTO, FfiError> {
        self.inner
            .initiate_sync(ForeignSyncTransport::new(transport))
            .map(Self::map_sync_session)
            .map_err(Into::into)
    }

    pub fn listen_sync(
        &self,
        transport: Box<dyn crate::SyncTransport>,
    ) -> Result<SyncSessionDTO, FfiError> {
        self.inner
            .listen_sync(ForeignSyncTransport::new(transport))
            .map(Self::map_sync_session)
            .map_err(Into::into)
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
    fn test_recent_note_page_is_exposed() {
        let service = open_memory().unwrap();
        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();

        let page_one = service
            .get_recent_notes_page(None, TimelineDirection::Older, Some(2))
            .unwrap();
        assert_eq!(page_one.notes.len(), 2);
        assert_eq!(page_one.notes[0].id, third.id);
        assert_eq!(page_one.notes[1].id, second.id);
        assert_eq!(page_one.next_cursor.as_deref(), Some(second.id.as_str()));

        let page_two = service
            .get_recent_notes_page(
                page_one.next_cursor.clone(),
                TimelineDirection::Older,
                Some(2),
            )
            .unwrap();
        assert_eq!(page_two.notes.len(), 1);
        assert_eq!(page_two.notes[0].id, first.id);
        assert!(page_two.next_cursor.is_none());
    }

    #[test]
    fn test_recent_sessions_page_is_exposed() {
        let service = open_memory().unwrap();
        let first = service.create_note("first".to_string(), vec![]).unwrap();
        let second = service.create_note("second".to_string(), vec![]).unwrap();
        let third = service.create_note("third".to_string(), vec![]).unwrap();

        let page = service.get_recent_sessions_page(None, Some(10)).unwrap();
        assert!(page.next_cursor.is_none());
        assert_eq!(page.sessions.len(), 1);
        assert_eq!(page.sessions[0].note_count, 3);
        assert_eq!(
            page.sessions[0]
                .notes
                .iter()
                .map(|note| note.id.clone())
                .collect::<Vec<_>>(),
            vec![third.id, second.id, first.id]
        );
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
    fn test_tag_queries_are_exposed() {
        let service = open_memory().unwrap();

        let first = service
            .create_note(
                "learn rust".to_string(),
                vec![
                    " rust ".to_string(),
                    "async".to_string(),
                    "rust".to_string(),
                ],
            )
            .unwrap();
        let second = service
            .create_note("ship rust".to_string(), vec!["rust".to_string()])
            .unwrap();
        let third = service
            .create_note("travel".to_string(), vec!["travel".to_string()])
            .unwrap();

        let tags = service.get_all_tags().unwrap();
        assert_eq!(
            tags,
            vec![
                "async".to_string(),
                "rust".to_string(),
                "travel".to_string(),
            ]
        );

        let page_one = service
            .get_notes_by_tag("rust".to_string(), None, Some(1))
            .unwrap();
        assert_eq!(page_one.len(), 1);
        assert_eq!(page_one[0].id, first.id);

        let page_two = service
            .get_notes_by_tag("rust".to_string(), Some(page_one[0].id.clone()), Some(10))
            .unwrap();
        assert_eq!(page_two.len(), 1);
        assert_eq!(page_two[0].id, second.id);

        let missing = service
            .get_notes_by_tag("missing".to_string(), None, None)
            .unwrap();
        assert!(missing.is_empty());

        let travel_notes = service
            .get_notes_by_tag("travel".to_string(), None, None)
            .unwrap();
        assert_eq!(travel_notes.len(), 1);
        assert_eq!(travel_notes[0].id, third.id);
    }

    #[test]
    fn test_sync_identity_and_peers_are_exposed() {
        let service = open_memory().unwrap();

        let identity = service.get_local_identity().unwrap();
        assert_eq!(identity.identity.algorithm, "x25519");
        assert_eq!(identity.identity.public_key.len(), 32);
        assert!(!identity.identity.kaomoji_fingerprint.is_empty());
        assert_eq!(identity.signing.algorithm, "ed25519");
        assert_eq!(identity.signing.public_key.len(), 32);
        assert!(!identity.signing.kaomoji_fingerprint.is_empty());

        let peers = service.get_peers().unwrap();
        assert!(peers.is_empty());
    }

    #[test]
    fn test_trust_peer_is_exposed() {
        let service = open_memory().unwrap();
        let peer_service = open_memory().unwrap();

        let peer_identity = peer_service.get_local_identity().unwrap();
        let peer = service
            .trust_peer(
                peer_identity.signing.public_key.clone(),
                Some("android phone".to_string()),
            )
            .unwrap();

        assert_eq!(peer.algorithm, "ed25519");
        assert_eq!(peer.public_key, peer_identity.signing.public_key);
        assert_eq!(peer.note.as_deref(), Some("android phone"));

        let peers = service.get_peers().unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], peer);
    }

    #[test]
    fn test_filtered_notes_are_exposed() {
        let service = open_memory().unwrap();

        let rust = service
            .create_note("rust".to_string(), vec!["rust".to_string()])
            .unwrap();
        let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
        let deleted = service
            .create_note("deleted rust".to_string(), vec!["rust".to_string()])
            .unwrap();
        service.delete_note(deleted.id.clone()).unwrap();

        let normal = service
            .get_filtered_notes(
                vec!["rust".to_string()],
                true,
                true,
                FilteredNoteStatus::Normal,
                None,
                Some(10),
            )
            .unwrap();
        assert_eq!(normal.len(), 2);
        assert_eq!(normal[0].id, untagged.id);
        assert_eq!(normal[1].id, rust.id);

        let all = service
            .get_filtered_notes(
                vec!["rust".to_string()],
                false,
                true,
                FilteredNoteStatus::All,
                None,
                Some(10),
            )
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, deleted.id);
        assert_eq!(all[1].id, rust.id);
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
