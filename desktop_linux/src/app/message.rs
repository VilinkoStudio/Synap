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
    // ── Browse navigation ──
    Navigate(ContentView),
    SearchChanged(String),
    LayoutChanged(NoteLayout),
    ClearFilters,

    // ── Focus mode ──
    OpenNoteFocus(String),
    NoteRowActivated(u32),
    ExitFocus,
    NoteDetailLoaded(Result<NoteDetailData, ServiceError>),

    // ── Context panel ──
    ToggleContextPanel,

    // ── Editing ──
    StartCreateNote,
    StartEditNote,
    StartReplyToNote,
    DraftContentChanged(String),
    DraftTagsChanged(String),
    SaveDraft,
    CancelDraft,

    // ── Note operations ──
    DeleteNote,
    EditNote,
    ReplyToNote,

    // ── Theme ──
    ThemeChanged(Theme),

    // ── List loading ──
    LoadMoreNotes,
    MoreNotesLoaded(Result<(Vec<NoteDTO>, Option<String>, bool), ServiceError>),

    // ── Tags ──
    TagSelected(String),
    TagsLoaded(Result<Vec<String>, ServiceError>),
    TagNotesLoaded(Result<Vec<NoteDTO>, ServiceError>),

    // ── Timeline ──
    TimelineLoaded(Result<Vec<synap_core::dto::TimelineSessionDTO>, ServiceError>),

    // ── Sync ──
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
    SyncSessionCompleted(Result<SyncSessionDTO, ServiceError>),
}
