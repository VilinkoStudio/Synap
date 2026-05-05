use crate::domain::{ContentView, NoteDetailData, NoteLayout, Theme};
use synap_core::{
    dto::{
        LocalIdentityDTO, NoteDTO, PeerDTO, PeerTrustStatusDTO, SyncSessionDTO,
        SyncSessionRecordDTO,
    },
    error::ServiceError,
};

#[derive(Debug)]
pub enum AppMsg {
    Navigate(ContentView),
    SearchChanged(String),
    LayoutChanged(NoteLayout),
    DeleteNote,
    SaveNote {
        id: Option<String>,
        content: String,
        tags: Vec<String>,
    },
    SaveReply {
        parent_id: String,
        content: String,
        tags: Vec<String>,
    },
    CreateNote,
    EditNote,
    ReplyToNote,
    ThemeChanged(Theme),
    NoteSelected(u32),
    NoteActivated(u32),
    NoteDetailLoaded(Result<NoteDetailData, ServiceError>),
    OpenNoteDetail(String),
    LoadMoreNotes,
    MoreNotesLoaded(Result<(Vec<NoteDTO>, Option<String>, bool), ServiceError>),
    TagSelected(String),
    TagsLoaded(Result<Vec<String>, ServiceError>),
    TagNotesLoaded(Result<Vec<NoteDTO>, ServiceError>),
    TagSuggestionsLoaded(Result<Vec<String>, ServiceError>),
    ClearFilters,
    TimelineLoaded(Result<Vec<synap_core::dto::TimelineSessionDTO>, ServiceError>),
    RefreshSync,
    SyncOverviewLoaded {
        listener: Result<corenet::ListenerState, ServiceError>,
        identity: Result<LocalIdentityDTO, ServiceError>,
        peers: Result<Vec<PeerDTO>, ServiceError>,
        sessions: Result<Vec<SyncSessionRecordDTO>, ServiceError>,
        discovered_peers: Vec<crate::domain::DiscoveredSyncPeer>,
        connections: Vec<crate::domain::SyncConnectionRecord>,
    },
    UpdateSyncHost(String),
    UpdateSyncPort(String),
    AddSyncConnection,
    DeleteSyncConnection(String),
    PairSyncConnection(String),
    PairDiscoveredPeer {
        host: String,
        port: u16,
    },
    TrustPeer {
        public_key: Vec<u8>,
        note: Option<String>,
    },
    UpdatePeerNote {
        peer_id: String,
        note: Option<String>,
    },
    SetPeerStatus {
        peer_id: String,
        status: PeerTrustStatusDTO,
    },
    DeletePeer(String),
    OpenPeer(String),
    UpdatePeerDraft(String),
    SyncSessionCompleted(Result<SyncSessionDTO, ServiceError>),
}
