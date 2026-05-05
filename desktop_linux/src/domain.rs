use synap_core::dto::{
    LocalIdentityDTO, NoteDTO, NoteVersionDTO, PeerDTO, PeerTrustStatusDTO, SyncSessionDTO,
    SyncSessionRecordDTO, SyncSessionRoleDTO, SyncStatusDTO,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentView {
    Notes,
    NoteDetail,
    Trash,
    Tags,
    TagNotes,
    Timeline,
    Settings,
}

impl ContentView {
    pub fn title(self) -> &'static str {
        match self {
            Self::Notes => "笔记列表",
            Self::NoteDetail => "笔记详情",
            Self::Trash => "回收站",
            Self::Tags => "标签",
            Self::TagNotes => "标签笔记",
            Self::Timeline => "时间线",
            Self::Settings => "设置",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::Auto,
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::Auto => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLayout {
    Waterfall,
    List,
}

impl NoteLayout {
    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::List,
            _ => Self::Waterfall,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Waterfall => "瀑布流",
            Self::List => "列表",
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::Waterfall => 0,
            Self::List => 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HomeData {
    pub notes: Vec<NoteDTO>,
    pub deleted_notes: Vec<NoteDTO>,
    pub notes_cursor: Option<String>,
    pub deleted_notes_cursor: Option<String>,
    pub has_more_notes: bool,
    pub has_more_deleted_notes: bool,
}

#[derive(Debug, Clone)]
pub struct NoteListItemViewModel {
    pub preview: String,
}

#[derive(Debug, Clone)]
pub struct NoteDetailViewModel {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at_label: String,
    pub deleted: bool,
}

#[derive(Debug, Clone)]
pub struct NoteDetailData {
    pub note: NoteDTO,
    pub replies: Vec<NoteDTO>,
    pub origins: Vec<NoteDTO>,
    pub other_versions: Vec<NoteVersionDTO>,
}

impl NoteDetailData {
    pub fn to_view_model(&self) -> NoteDetailViewModel {
        NoteDetailViewModel {
            id: self.note.id.clone(),
            content: self.note.content.clone(),
            tags: self.note.tags.clone(),
            created_at_label: format_timestamp(self.note.created_at),
            deleted: self.note.deleted,
        }
    }
}

impl From<&NoteDTO> for NoteListItemViewModel {
    fn from(value: &NoteDTO) -> Self {
        Self {
            preview: build_preview(&value.content),
        }
    }
}

impl From<&NoteDTO> for NoteDetailViewModel {
    fn from(value: &NoteDTO) -> Self {
        Self {
            id: value.id.clone(),
            content: value.content.clone(),
            tags: value.tags.clone(),
            created_at_label: format_timestamp(value.created_at),
            deleted: value.deleted,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub home: HomeData,
    pub search_query: String,
    pub content_view: ContentView,
    pub layout: NoteLayout,
    pub selected_note_id: Option<String>,
    pub selected_note_detail: Option<NoteDetailViewModel>,
    pub selected_note_full: Option<NoteDetailData>,
    pub status: Option<String>,
    pub theme: Theme,
    pub is_loading_more: bool,
    pub selected_tag: Option<String>,
    pub tag_notes: Vec<NoteDTO>,
    pub all_tags: Vec<String>,
    pub tag_suggestions: Vec<String>,
    pub timeline_sessions: Vec<synap_core::dto::TimelineSessionDTO>,
    pub timeline_cursor: Option<String>,
    pub has_more_timeline: bool,
    pub sync: SyncState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            home: HomeData::default(),
            search_query: String::new(),
            content_view: ContentView::Notes,
            layout: NoteLayout::Waterfall,
            selected_note_id: None,
            selected_note_detail: None,
            selected_note_full: None,
            status: None,
            theme: Theme::default(),
            is_loading_more: false,
            selected_tag: None,
            tag_notes: Vec::new(),
            all_tags: Vec::new(),
            tag_suggestions: Vec::new(),
            timeline_sessions: Vec::new(),
            timeline_cursor: None,
            has_more_timeline: false,
            sync: SyncState::default(),
        }
    }
}

impl AppState {
    pub fn visible_notes(&self) -> Vec<NoteDTO> {
        match self.content_view {
            ContentView::Notes => self.home.notes.clone(),
            ContentView::NoteDetail => self
                .selected_note_detail
                .as_ref()
                .map(|detail| {
                    if detail.deleted {
                        self.home.deleted_notes.clone()
                    } else {
                        self.home.notes.clone()
                    }
                })
                .unwrap_or_else(|| self.home.notes.clone()),
            ContentView::Trash => {
                filter_deleted_notes(&self.home.deleted_notes, &self.search_query)
            }
            ContentView::Tags => self.home.notes.clone(),
            ContentView::TagNotes => self.tag_notes.clone(),
            ContentView::Timeline => Vec::new(),
            ContentView::Settings => Vec::new(),
        }
    }

    pub fn sync_selection(&mut self) {
        if self.content_view == ContentView::Settings {
            return;
        }

        let visible = self.visible_notes();
        let is_selected_visible = self
            .selected_note_id
            .as_ref()
            .is_some_and(|selected_id| visible.iter().any(|note| note.id == *selected_id));

        if !is_selected_visible {
            self.selected_note_id = visible.first().map(|note| note.id.clone());
        }

        self.selected_note_detail = self
            .selected_note_id
            .as_ref()
            .and_then(|selected_id| visible.iter().find(|note| note.id == *selected_id))
            .map(NoteDetailViewModel::from);

        // If we have full detail data but the note changed, clear it
        if let Some(full) = &self.selected_note_full {
            if self.selected_note_id.as_deref() != Some(&full.note.id) {
                self.selected_note_full = None;
            }
        }
    }

    pub fn selected_index_in(&self, notes: &[NoteDTO]) -> Option<usize> {
        let selected = self.selected_note_id.as_ref()?;
        notes.iter().position(|note| note.id == *selected)
    }
}

fn filter_deleted_notes(notes: &[NoteDTO], query: &str) -> Vec<NoteDTO> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return notes.to_vec();
    }

    let needle = trimmed.to_lowercase();
    notes
        .iter()
        .filter(|note| {
            note.content.to_lowercase().contains(&needle)
                || note
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect()
}

fn build_preview(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return "空白笔记".to_string();
    }

    let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_CHARS: usize = 220;

    if normalized.chars().count() <= MAX_CHARS {
        normalized
    } else {
        let preview: String = normalized.chars().take(MAX_CHARS).collect();
        format!("{preview}...")
    }
}

pub fn format_timestamp(timestamp_ms: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let timestamp = UNIX_EPOCH + Duration::from_millis(timestamp_ms);
    let datetime = chrono::DateTime::<chrono::Local>::from(timestamp);
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub is_loading: bool,
    pub is_pairing: bool,
    pub is_managing_peer: bool,
    pub listener: SyncListenerState,
    pub local_identity: Option<LocalIdentityDTO>,
    pub discovered_peers: Vec<DiscoveredSyncPeer>,
    pub connections: Vec<SyncConnectionRecord>,
    pub peers: Vec<PeerDTO>,
    pub pending_trust_peer: Option<PeerDTO>,
    pub recent_sessions: Vec<SyncSessionRecordDTO>,
    pub error_message: Option<String>,
    pub host_input: String,
    pub port_input: String,
    pub peer_note_draft: String,
    pub active_peer_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SyncListenerState {
    pub protocol: String,
    pub backend: String,
    pub is_listening: bool,
    pub listen_port: Option<u16>,
    pub local_addresses: Vec<String>,
    pub status: String,
    pub error_message: Option<String>,
}

impl From<corenet::ListenerState> for SyncListenerState {
    fn from(value: corenet::ListenerState) -> Self {
        Self {
            protocol: value.protocol,
            backend: value.backend,
            is_listening: value.is_listening,
            listen_port: value.listen_port,
            local_addresses: value.local_addresses,
            status: value.status,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncConnectionStatus {
    Idle,
    Connecting,
    AwaitingTrust,
    Connected,
    Failed,
}

#[derive(Debug, Clone)]
pub struct SyncConnectionRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: SyncConnectionStatus,
    pub status_message: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredSyncPeer {
    pub service_name: String,
    pub display_name: String,
    pub host: String,
    pub port: u16,
    pub last_seen_at_ms: u64,
}

impl From<corenet::DiscoveredPeer> for DiscoveredSyncPeer {
    fn from(value: corenet::DiscoveredPeer) -> Self {
        Self {
            service_name: value.service_name,
            display_name: value.display_name,
            host: value.host,
            port: value.port,
            last_seen_at_ms: value.last_seen_at_ms,
        }
    }
}

pub fn sync_status_label(status: &SyncStatusDTO) -> &'static str {
    match status {
        SyncStatusDTO::Completed => "已完成",
        SyncStatusDTO::PendingTrust => "待信任",
        SyncStatusDTO::Failed => "失败",
    }
}

pub fn sync_role_label(role: &SyncSessionRoleDTO) -> &'static str {
    match role {
        SyncSessionRoleDTO::Initiator => "发起方",
        SyncSessionRoleDTO::Listener => "监听方",
    }
}

pub fn peer_status_label(status: &PeerTrustStatusDTO) -> &'static str {
    match status {
        PeerTrustStatusDTO::Pending => "待确认",
        PeerTrustStatusDTO::Trusted => "已信任",
        PeerTrustStatusDTO::Retired => "已停用",
        PeerTrustStatusDTO::Revoked => "已撤销",
    }
}

pub fn sync_session_summary(session: &SyncSessionDTO) -> String {
    let peer_label = session
        .peer
        .note
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| session.peer.kaomoji_fingerprint.clone());
    format!("{} · {}", peer_label, sync_status_label(&session.status))
}
