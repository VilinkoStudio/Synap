//! Sync operations — connection management, pairing, peer trust.

use relm4::prelude::*;

use super::App;
use super::message::AppMsg;

impl App {
    pub(super) fn refresh_sync(&mut self, sender: &ComponentSender<Self>) {
        self.state.sync.is_loading = true;
        self.state.sync.error_message = None;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let listener = core.ensure_sync_listener_started(45_172);
            let identity = core.get_local_identity();
            let peers = core.get_peers();
            let sessions = core.get_recent_sync_sessions(Some(10));
            let discovered_peers = core
                .discovered_sync_peers()
                .into_iter()
                .map(Into::into)
                .collect();
            let connections = core.sync_connections();
            let _ = sender.input_sender().send(AppMsg::SyncOverviewLoaded {
                listener,
                identity,
                peers,
                sessions,
                discovered_peers,
                connections,
            });
        });
    }

    pub(super) fn finish_refresh_sync(
        &mut self,
        listener: Result<corenet::ListenerState, synap_core::error::ServiceError>,
        identity: Result<synap_core::dto::LocalIdentityDTO, synap_core::error::ServiceError>,
        peers: Result<Vec<synap_core::dto::PeerDTO>, synap_core::error::ServiceError>,
        sessions: Result<
            Vec<synap_core::dto::SyncSessionRecordDTO>,
            synap_core::error::ServiceError,
        >,
        discovered_peers: Vec<crate::domain::DiscoveredSyncPeer>,
        connections: Vec<crate::domain::SyncConnectionRecord>,
    ) {
        self.state.sync.is_loading = false;
        self.state.sync.discovered_peers = discovered_peers;
        self.state.sync.connections = connections;
        let mut errors = Vec::new();
        match listener {
            Ok(v) => self.state.sync.listener = v.into(),
            Err(e) => errors.push(format!("监听失败: {e}")),
        }
        match identity {
            Ok(v) => self.state.sync.local_identity = Some(v),
            Err(e) => errors.push(format!("读取本机身份失败: {e}")),
        }
        match peers {
            Ok(v) => self.state.sync.peers = v,
            Err(e) => errors.push(format!("读取设备列表失败: {e}")),
        }
        match sessions {
            Ok(v) => self.state.sync.recent_sessions = v,
            Err(e) => errors.push(format!("读取同步统计失败: {e}")),
        }
        self.state.sync.error_message = (!errors.is_empty()).then(|| errors.join("\n"));
    }

    pub(super) fn add_sync_connection(&mut self) {
        let host = self.state.sync.host_input.trim().to_string();
        let port = self.state.sync.port_input.trim().parse::<u16>();
        match port {
            Ok(port) => match self.core.save_sync_connection(&host, port) {
                Ok(record) => {
                    self.state.sync.connections.retain(|c| c.id != record.id);
                    self.state.sync.connections.push(record);
                    self.state.sync.host_input.clear();
                    self.state.sync.port_input.clear();
                    self.state.sync.error_message = None;
                }
                Err(e) => self.state.sync.error_message = Some(format!("保存连接失败: {e}")),
            },
            Err(_) => self.state.sync.error_message = Some("端口必须是有效数字".to_string()),
        }
    }

    pub(super) fn delete_sync_connection(&mut self, id: &str) {
        match self.core.delete_sync_connection(id) {
            Ok(()) => {
                self.state.sync.connections.retain(|c| c.id != id);
                self.state.sync.error_message = None;
            }
            Err(e) => self.state.sync.error_message = Some(format!("删除连接失败: {e}")),
        }
    }

    pub(super) fn start_sync_pair(&mut self, host: String, port: u16, sender: &ComponentSender<Self>) {
        self.state.sync.is_pairing = true;
        self.state.sync.error_message = None;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.connect_and_sync(&host, port);
            let _ = sender
                .input_sender()
                .send(AppMsg::SyncSessionCompleted(result));
        });
    }

    pub(super) fn finish_sync_pair(
        &mut self,
        result: Result<synap_core::dto::SyncSessionDTO, synap_core::error::ServiceError>,
    ) {
        self.state.sync.is_pairing = false;
        match result {
            Ok(session) => {
                self.state.sync.pending_trust_peer = (session.status
                    == synap_core::dto::SyncStatusDTO::PendingTrust)
                    .then_some(session.peer.clone());
                self.state.sync.error_message = None;
            }
            Err(e) => self.state.sync.error_message = Some(format!("配对失败: {e}")),
        }
    }

    pub(super) fn trust_peer(&mut self, public_key: Vec<u8>, note: Option<String>) {
        self.state.sync.is_managing_peer = true;
        match self.core.trust_peer(&public_key, note) {
            Ok(peer) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.pending_trust_peer = None;
                self.state.sync.peers.retain(|p| p.id != peer.id);
                self.state.sync.peers.push(peer);
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("信任对端失败: {e}"));
            }
        }
    }

    pub(super) fn update_peer_note(&mut self, peer_id: String, note: Option<String>) {
        self.state.sync.is_managing_peer = true;
        match self.core.update_peer_note(&peer_id, note) {
            Ok(peer) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.peers.retain(|p| p.id != peer.id);
                self.state.sync.peers.push(peer.clone());
                self.state.sync.peer_note_draft = peer.note.clone().unwrap_or_default();
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("更新设备备注失败: {e}"));
            }
        }
    }

    pub(super) fn delete_peer(&mut self, peer_id: String) {
        self.state.sync.is_managing_peer = true;
        match self.core.delete_peer(&peer_id) {
            Ok(()) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.peers.retain(|p| p.id != peer_id);
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("删除设备失败: {e}"));
            }
        }
    }
}
