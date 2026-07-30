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

    pub(super) fn add_sync_connection(&mut self, sender: &ComponentSender<Self>) {
        let host = self.state.sync.host_input.trim().to_string();
        let port = self.state.sync.port_input.trim().parse::<u16>();
        match port {
            Ok(port) => {
                let core = self.core.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let result = core.save_sync_connection(&host, port);
                    let _ = sender
                        .input_sender()
                        .send(AppMsg::SyncConnectionSaved(result));
                });
            }
            Err(_) => self.state.sync.error_message = Some("端口必须是有效数字".to_string()),
        }
    }

    pub(super) fn delete_sync_connection(&mut self, id: &str, sender: &ComponentSender<Self>) {
        // Optimistic removal from local state
        self.state.sync.connections.retain(|c| c.id != id);
        self.state.sync.error_message = None;

        let core = self.core.clone();
        let id = id.to_string();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.delete_sync_connection(&id);
            let _ = sender
                .input_sender()
                .send(AppMsg::SyncConnectionDeleted(result));
        });
    }

    pub(super) fn start_sync_pair(
        &mut self,
        host: String,
        port: u16,
        sender: &ComponentSender<Self>,
    ) {
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

    pub(super) fn trust_peer(
        &mut self,
        public_key: Vec<u8>,
        note: Option<String>,
        sender: &ComponentSender<Self>,
    ) {
        self.state.sync.is_managing_peer = true;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.trust_peer(&public_key, note);
            let _ = sender.input_sender().send(AppMsg::PeerTrusted(result));
        });
    }

    pub(super) fn update_peer_note(
        &mut self,
        peer_id: String,
        note: Option<String>,
        sender: &ComponentSender<Self>,
    ) {
        self.state.sync.is_managing_peer = true;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.update_peer_note(&peer_id, note);
            let _ = sender.input_sender().send(AppMsg::PeerNoteUpdated(result));
        });
    }

    pub(super) fn delete_peer(&mut self, peer_id: String, sender: &ComponentSender<Self>) {
        self.state.sync.is_managing_peer = true;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.delete_peer(&peer_id);
            let _ = sender.input_sender().send(AppMsg::PeerDeleted(result));
        });
    }

    pub(super) fn set_peer_status(
        &mut self,
        peer_id: String,
        status: synap_core::dto::PeerTrustStatusDTO,
        sender: &ComponentSender<Self>,
    ) {
        self.state.sync.is_managing_peer = true;
        // For desktop, set_peer_status is equivalent to trust_peer when status is Trusted
        // For other statuses, we'd need a dedicated API. For now, handle Trusted case.
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = match status {
                synap_core::dto::PeerTrustStatusDTO::Trusted => {
                    // Get the peer's public key and trust them
                    match core.get_peers() {
                        Ok(peers) => {
                            if let Some(peer) = peers.iter().find(|p| p.id == peer_id) {
                                let pk = peer.public_key.clone();
                                let note = peer.note.clone();
                                core.trust_peer(&pk, note)
                            } else {
                                Err(synap_core::error::ServiceError::NotFound(format!(
                                    "设备 {peer_id} 未找到"
                                )))
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                _ => {
                    // For other statuses, just refresh peers
                    Err(synap_core::error::ServiceError::Other(anyhow::anyhow!(
                        "暂不支持设置此设备状态"
                    )))
                }
            };
            let _ = sender.input_sender().send(AppMsg::PeerTrusted(result));
        });
    }

    pub(super) fn dismiss_pending_trust_prompt(&mut self) {
        self.state.sync.pending_trust_peer = None;
    }

    // ── Relay operations ──

    pub(super) fn update_relay_base_url(&mut self, value: &str) {
        self.state.sync.relay_base_url = value.to_string();
        self.state.sync.relay_status_message = None;
    }

    pub(super) fn update_relay_api_key(&mut self, value: &str) {
        self.state.sync.relay_api_key = value.to_string();
        self.state.sync.relay_status_message = None;
    }

    pub(super) fn save_relay_config(&mut self, sender: &ComponentSender<Self>) {
        self.state.sync.is_relay_syncing = true;
        self.state.sync.error_message = None;
        self.state.sync.relay_status_message = None;
        let core = self.core.clone();
        let base_url = self.state.sync.relay_base_url.clone();
        let api_key = self.state.sync.relay_api_key.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.save_relay_config(&base_url, &api_key);
            let _ = sender.input_sender().send(AppMsg::RelayConfigSaved(result));
        });
    }

    pub(super) fn finish_save_relay_config(
        &mut self,
        result: Result<(), synap_core::error::ServiceError>,
    ) {
        self.state.sync.is_relay_syncing = false;
        match result {
            Ok(()) => {
                self.state.sync.relay_status_message = Some("Relay 配置已保存".to_string());
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.error_message = Some(format!("保存 Relay 配置失败: {e}"));
            }
        }
    }

    pub(super) fn fetch_relay_updates(&mut self, sender: &ComponentSender<Self>) {
        self.state.sync.is_relay_syncing = true;
        self.state.sync.error_message = None;
        self.state.sync.relay_status_message = None;
        // Auto-save config before fetching (matches Android behavior)
        let base_url = self.state.sync.relay_base_url.clone();
        let api_key = self.state.sync.relay_api_key.clone();
        let _ = self.core.save_relay_config(&base_url, &api_key);
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.relay_fetch_updates();
            let _ = sender
                .input_sender()
                .send(AppMsg::RelayFetchCompleted(result));
        });
    }

    pub(super) fn finish_fetch_relay_updates(
        &mut self,
        result: Result<synap_core::dto::RelayFetchStatsDTO, synap_core::error::ServiceError>,
    ) {
        self.state.sync.is_relay_syncing = false;
        match result {
            Ok(stats) => {
                self.state.sync.relay_status_message = Some(format!(
                    "拉取完成：获取 {} 封，导入 {} 封",
                    stats.fetched_messages, stats.imported_messages
                ));
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.error_message = Some(format!("Relay 拉取失败: {e}"));
            }
        }
    }

    pub(super) fn push_relay_updates(&mut self, sender: &ComponentSender<Self>) {
        self.state.sync.is_relay_syncing = true;
        self.state.sync.error_message = None;
        self.state.sync.relay_status_message = None;
        // Auto-save config before pushing (matches Android behavior)
        let base_url = self.state.sync.relay_base_url.clone();
        let api_key = self.state.sync.relay_api_key.clone();
        let _ = self.core.save_relay_config(&base_url, &api_key);
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.relay_push_updates();
            let _ = sender
                .input_sender()
                .send(AppMsg::RelayPushCompleted(result));
        });
    }

    pub(super) fn finish_push_relay_updates(
        &mut self,
        result: Result<synap_core::dto::RelayPushStatsDTO, synap_core::error::ServiceError>,
    ) {
        self.state.sync.is_relay_syncing = false;
        match result {
            Ok(stats) => {
                self.state.sync.relay_status_message = Some(format!(
                    "推送完成：投递 {}/{} 个设备",
                    stats.posted_messages, stats.trusted_peers
                ));
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.error_message = Some(format!("Relay 推送失败: {e}"));
            }
        }
    }
}
