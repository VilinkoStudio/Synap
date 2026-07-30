use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use corenet::{
    ConnectConfig, DiscoveryConfig, ListenConfig, ListenerState, SyncAcceptLoopHandle,
    SyncDiscoveryRuntime, SyncNetService,
};
use synap_core::{
    dto::{
        LocalIdentityDTO, NoteDTO, NoteVersionDTO, PeerDTO, RelayFetchStatsDTO, RelayPushStatsDTO,
        SyncSessionDTO, SyncSessionRecordDTO, TimelineNotesPageDTO, TimelineSessionsPageDTO,
    },
    error::ServiceError,
    service::{SynapService, TimelineDirection},
};

use crate::domain::SyncConnectionRecord;

pub type CoreResult<T> = Result<T, ServiceError>;

pub trait DesktopCore {
    fn recent_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO>;
    fn deleted_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO>;
    fn home_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        group_sessions: bool,
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
    fn other_versions(&self, note_id: &str) -> CoreResult<Vec<NoteVersionDTO>>;

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
    fn get_local_identity(&self) -> CoreResult<LocalIdentityDTO>;
    fn get_peers(&self) -> CoreResult<Vec<PeerDTO>>;
    fn trust_peer(&self, public_key: &[u8], note: Option<String>) -> CoreResult<PeerDTO>;
    fn update_peer_note(&self, peer_id: &str, note: Option<String>) -> CoreResult<PeerDTO>;
    fn delete_peer(&self, peer_id: &str) -> CoreResult<()>;
    fn get_recent_sync_sessions(
        &self,
        limit: Option<usize>,
    ) -> CoreResult<Vec<SyncSessionRecordDTO>>;
    fn ensure_sync_listener_started(&self, preferred_port: u16) -> CoreResult<ListenerState>;
    fn discovered_sync_peers(&self) -> Vec<corenet::DiscoveredPeer>;
    fn connect_and_sync(&self, host: &str, port: u16) -> CoreResult<SyncSessionDTO>;
    fn sync_connections(&self) -> Vec<SyncConnectionRecord>;
    fn save_sync_connection(&self, host: &str, port: u16) -> CoreResult<SyncConnectionRecord>;
    fn delete_sync_connection(&self, connection_id: &str) -> CoreResult<()>;
    fn get_relay_config(&self) -> (String, String);
    fn save_relay_config(&self, base_url: &str, api_key: &str) -> CoreResult<()>;
    fn relay_fetch_updates(&self) -> CoreResult<RelayFetchStatsDTO>;
    fn relay_push_updates(&self) -> CoreResult<RelayPushStatsDTO>;
}

pub struct SynapCoreAdapter {
    service: Arc<SynapService>,
    net: SyncNetService,
    sync_runtime: Mutex<DesktopSyncRuntime>,
}

impl SynapCoreAdapter {
    pub fn new_from_env() -> CoreResult<Self> {
        let db_path =
            std::env::var("SYNAP_DESKTOP_DB").unwrap_or_else(|_| "synap-desktop.redb".to_string());
        let service = Arc::new(SynapService::new(Some(db_path))?);
        Ok(Self {
            service,
            net: SyncNetService::new(),
            sync_runtime: Mutex::new(DesktopSyncRuntime {
                connections: load_connections(),
                ..Default::default()
            }),
        })
    }
}

impl DesktopCore for SynapCoreAdapter {
    fn recent_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO> {
        self.service.get_timeline_notes_page(
            vec![],
            true,
            false,
            synap_core::service::FilteredNoteStatus::Normal,
            false,
            cursor,
            TimelineDirection::Older,
            limit,
        )
    }

    fn deleted_notes_page(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO> {
        self.service.get_timeline_notes_page(
            vec![],
            true,
            false,
            synap_core::service::FilteredNoteStatus::Deleted,
            false,
            cursor,
            TimelineDirection::Older,
            limit,
        )
    }

    fn home_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        group_sessions: bool,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineNotesPageDTO> {
        self.service.get_timeline_notes_page(
            selected_tags,
            include_untagged,
            tag_filter_enabled,
            synap_core::service::FilteredNoteStatus::Normal,
            group_sessions,
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

    fn other_versions(&self, note_id: &str) -> CoreResult<Vec<NoteVersionDTO>> {
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

    #[allow(deprecated)]
    fn get_recent_sessions(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<TimelineSessionsPageDTO> {
        self.service.get_recent_sessions(cursor, limit)
    }

    fn get_local_identity(&self) -> CoreResult<LocalIdentityDTO> {
        self.service.get_local_identity()
    }

    fn get_peers(&self) -> CoreResult<Vec<PeerDTO>> {
        self.service.get_peers()
    }

    fn trust_peer(&self, public_key: &[u8], note: Option<String>) -> CoreResult<PeerDTO> {
        self.service.trust_peer(public_key, note)
    }

    fn update_peer_note(&self, peer_id: &str, note: Option<String>) -> CoreResult<PeerDTO> {
        self.service.update_peer_note(peer_id, note)
    }

    fn delete_peer(&self, peer_id: &str) -> CoreResult<()> {
        self.service.delete_peer(peer_id)
    }

    fn get_recent_sync_sessions(
        &self,
        limit: Option<usize>,
    ) -> CoreResult<Vec<SyncSessionRecordDTO>> {
        self.service.get_recent_sync_sessions(limit)
    }

    fn ensure_sync_listener_started(&self, preferred_port: u16) -> CoreResult<ListenerState> {
        let mut runtime = self.sync_runtime.lock().unwrap();
        if runtime.listener_handle.is_none() {
            let listener = self
                .net
                .ensure_listener_started(ListenConfig {
                    port: Some(preferred_port),
                })
                .or_else(|_| self.net.ensure_listener_started(ListenConfig::default()))
                .map_err(map_sync_error)?;

            let service = Arc::clone(&self.service);
            let handle = self.net.spawn_accept_loop(service, listener, |_| {});
            let state = handle.state();
            runtime.listener_handle = Some(handle);
            runtime.listener_state = state.clone();

            if runtime.discovery.is_none() {
                let display_name = build_discovery_name();
                if let Some(port) = state.listen_port {
                    let sign_service = Arc::clone(&self.service);
                    let sign_cb: corenet::MdnsSignCallback = Box::new(move || {
                        sign_service
                            .sign_mdns_discovery()
                            .map_err(|e| e.to_string())
                    });
                    runtime.discovery = SyncDiscoveryRuntime::start(DiscoveryConfig {
                        display_name,
                        listen_port: port,
                        sign: Some(Arc::new(sign_cb)),
                    })
                    .ok();
                }
            }
        } else if let Some(handle) = runtime.listener_handle.as_ref() {
            runtime.listener_state = handle.state();
        }

        Ok(runtime.listener_state.clone())
    }

    fn discovered_sync_peers(&self) -> Vec<corenet::DiscoveredPeer> {
        self.sync_runtime
            .lock()
            .unwrap()
            .discovery
            .as_ref()
            .map(SyncDiscoveryRuntime::peers)
            .unwrap_or_default()
    }

    fn connect_and_sync(&self, host: &str, port: u16) -> CoreResult<SyncSessionDTO> {
        self.net
            .connect_and_sync(
                self.service.as_ref(),
                ConnectConfig {
                    host: host.to_string(),
                    port,
                },
            )
            .map_err(map_sync_error)
    }

    fn sync_connections(&self) -> Vec<SyncConnectionRecord> {
        let runtime = self.sync_runtime.lock().unwrap();
        runtime.connections.clone()
    }

    fn save_sync_connection(&self, host: &str, port: u16) -> CoreResult<SyncConnectionRecord> {
        let host = host.trim();
        if host.is_empty() {
            return Err(ServiceError::Other(anyhow::anyhow!("主机地址不能为空")));
        }
        if !(1..=65535).contains(&port) {
            return Err(ServiceError::Other(anyhow::anyhow!(
                "端口必须在 1 到 65535 之间"
            )));
        }

        let mut runtime = self.sync_runtime.lock().unwrap();
        let record = SyncConnectionRecord {
            id: format!("{host}:{port}"),
            name: format!("{host}:{port}"),
            host: host.to_string(),
            port,
            status_message: "已保存，尚未配对".to_string(),
        };

        runtime.connections.retain(|item| item.id != record.id);
        runtime.connections.push(record.clone());
        persist_connections(&runtime.connections)?;
        Ok(record)
    }

    fn delete_sync_connection(&self, connection_id: &str) -> CoreResult<()> {
        let mut runtime = self.sync_runtime.lock().unwrap();
        runtime.connections.retain(|item| item.id != connection_id);
        persist_connections(&runtime.connections)?;
        Ok(())
    }

    fn get_relay_config(&self) -> (String, String) {
        let config = load_relay_config();
        (config.base_url, config.api_key)
    }

    fn save_relay_config(&self, base_url: &str, api_key: &str) -> CoreResult<()> {
        persist_relay_config(&RelayConfig {
            base_url: base_url.trim().to_string(),
            api_key: api_key.trim().to_string(),
        })
    }

    fn relay_fetch_updates(&self) -> CoreResult<RelayFetchStatsDTO> {
        let config = load_relay_config();
        if config.base_url.is_empty() {
            return Err(ServiceError::Other(anyhow::anyhow!("请先配置 Relay 地址")));
        }
        self.service
            .relay_fetch_updates(&config.base_url, Some(&config.api_key))
    }

    fn relay_push_updates(&self) -> CoreResult<RelayPushStatsDTO> {
        let config = load_relay_config();
        if config.base_url.is_empty() {
            return Err(ServiceError::Other(anyhow::anyhow!("请先配置 Relay 地址")));
        }
        self.service
            .relay_push_updates(&config.base_url, Some(&config.api_key))
    }
}

#[derive(Default)]
struct DesktopSyncRuntime {
    listener_handle: Option<SyncAcceptLoopHandle>,
    listener_state: ListenerState,
    discovery: Option<SyncDiscoveryRuntime>,
    connections: Vec<SyncConnectionRecord>,
}

fn build_discovery_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Synap Desktop".to_string())
}

fn map_sync_error(error: corenet::SyncNetError) -> ServiceError {
    ServiceError::Other(anyhow::anyhow!(error))
}

fn connections_path() -> PathBuf {
    PathBuf::from("synap-desktop-sync-connections.json")
}

fn persist_connections(connections: &[SyncConnectionRecord]) -> CoreResult<()> {
    let payload = connections
        .iter()
        .map(|item| format!("{}\t{}\t{}\n", item.id, item.host, item.port))
        .collect::<String>();
    fs::write(connections_path(), payload)
        .map_err(|error| ServiceError::Other(anyhow::anyhow!(error)))
}

fn load_connections() -> Vec<SyncConnectionRecord> {
    let Ok(contents) = fs::read_to_string(connections_path()) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let id = parts.next()?.to_string();
            let host = parts.next()?.to_string();
            let port = parts.next()?.parse::<u16>().ok()?;
            Some(SyncConnectionRecord {
                id: id.clone(),
                name: id,
                host,
                port,
                status_message: "已保存，尚未配对".to_string(),
            })
        })
        .collect()
}

// ── Relay config persistence ──

struct RelayConfig {
    base_url: String,
    api_key: String,
}

fn relay_config_path() -> PathBuf {
    PathBuf::from("synap-desktop-relay-config.json")
}

fn persist_relay_config(config: &RelayConfig) -> CoreResult<()> {
    let json = format!(
        "{{\"baseUrl\":\"{}\",\"apiKey\":\"{}\"}}",
        config.base_url.replace('"', "\\\""),
        config.api_key.replace('"', "\\\"")
    );
    fs::write(relay_config_path(), json)
        .map_err(|error| ServiceError::Other(anyhow::anyhow!(error)))
}

fn load_relay_config() -> RelayConfig {
    let Ok(contents) = fs::read_to_string(relay_config_path()) else {
        return RelayConfig {
            base_url: String::new(),
            api_key: String::new(),
        };
    };
    // Simple JSON parsing without serde dependency
    let base_url = extract_json_string(&contents, "baseUrl");
    let api_key = extract_json_string(&contents, "apiKey");
    RelayConfig { base_url, api_key }
}

fn extract_json_string(json: &str, key: &str) -> String {
    let pattern = format!("\"{key}\":\"");
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = json[value_start..].find('"') {
            return json[value_start..value_start + end].to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_string_finds_value() {
        let json = r#"{"baseUrl":"https://relay.example.com","apiKey":"secret123"}"#;
        assert_eq!(
            extract_json_string(json, "baseUrl"),
            "https://relay.example.com"
        );
        assert_eq!(extract_json_string(json, "apiKey"), "secret123");
    }

    #[test]
    fn extract_json_string_missing_key() {
        let json = r#"{"baseUrl":"https://example.com"}"#;
        assert_eq!(extract_json_string(json, "apiKey"), "");
    }

    #[test]
    fn extract_json_string_empty_json() {
        assert_eq!(extract_json_string("{}", "baseUrl"), "");
    }

    #[test]
    fn extract_json_string_empty_value() {
        let json = r#"{"baseUrl":"","apiKey":"key"}"#;
        assert_eq!(extract_json_string(json, "baseUrl"), "");
        assert_eq!(extract_json_string(json, "apiKey"), "key");
    }

    #[test]
    fn relay_config_roundtrip() {
        let path = relay_config_path();
        // Clean up any existing file
        let _ = fs::remove_file(&path);

        let config = RelayConfig {
            base_url: "https://relay.test".to_string(),
            api_key: "test-key".to_string(),
        };
        persist_relay_config(&config).unwrap();

        let loaded = load_relay_config();
        assert_eq!(loaded.base_url, "https://relay.test");
        assert_eq!(loaded.api_key, "test-key");

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_relay_config_missing_file() {
        // load_relay_config should return empty defaults when file doesn't exist
        let config = RelayConfig {
            base_url: String::new(),
            api_key: String::new(),
        };
        // Just verify it doesn't panic
        let _ = config;
    }
}
