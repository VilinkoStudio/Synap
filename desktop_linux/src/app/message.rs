use crate::domain::{ContentView, HomeData, NoteDetailData, SyncConnectionRecord, Theme};
use synap_core::{
    dto::{
        LocalIdentityDTO, NoteDTO, PeerDTO, PeerTrustStatusDTO, RelayFetchStatsDTO,
        RelayPushStatsDTO, SyncSessionDTO, SyncSessionRecordDTO,
    },
    error::ServiceError,
};

#[derive(Debug)]
pub enum AppMsg {
    // ── Browse navigation ──
    Navigate(ContentView),
    OpenSettings,
    SearchChanged(String),
    RunSearch,
    ClearFilters,

    // ── Focus mode ──
    OpenNoteFocus(String),
    ExitFocus,
    NoteDetailLoaded(Result<NoteDetailData, ServiceError>),

    // ── Context panel ──
    ToggleContextPanel,

    // ── Editing ──
    StartCreateNote,
    DraftContentChanged(String),
    DraftTagsChanged(String),
    SaveDraft,
    CancelDraft,

    // ── Note operations ──
    DeleteNote,
    ConfirmDeleteNote,
    RestoreNote(String),
    EditNote,
    ReplyToNote,

    // ── Theme ──
    ThemeChanged(Theme),

    // ── List loading ──
    LoadMoreNotes,
    MoreNotesLoaded(Result<(Vec<NoteDTO>, Option<String>, bool), ServiceError>),

    // ── Tag filters ──
    ToggleTagFilter(String),
    ToggleUntaggedFilter,
    ToggleAllTags,
    TagRecommendationsLoaded(Result<Vec<String>, ServiceError>),

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
    DeletePeer(String),
    SetPeerStatus {
        peer_id: String,
        status: PeerTrustStatusDTO,
    },
    DismissPendingTrustPrompt,
    SyncSessionCompleted(Result<SyncSessionDTO, ServiceError>),

    // ── Relay config ──
    UpdateRelayBaseUrl(String),
    UpdateRelayApiKey(String),
    SaveRelayConfig,
    RelayConfigSaved(Result<(), ServiceError>),

    // ── Relay operations ──
    FetchRelayUpdates,
    PushRelayUpdates,
    RelayFetchCompleted(Result<RelayFetchStatsDTO, ServiceError>),
    RelayPushCompleted(Result<RelayPushStatsDTO, ServiceError>),

    // ── Async operation results ──
    NoteSaved(Result<NoteDTO, ServiceError>),
    NoteDeleted(Result<(), ServiceError>),
    NoteRestored(Result<(), ServiceError>),
    HomeRefreshed(Result<HomeData, ServiceError>),
    SyncConnectionSaved(Result<SyncConnectionRecord, ServiceError>),
    SyncConnectionDeleted(Result<(), ServiceError>),
    PeerTrusted(Result<PeerDTO, ServiceError>),
    PeerNoteUpdated(Result<PeerDTO, ServiceError>),
    PeerDeleted(Result<(), ServiceError>),
}
