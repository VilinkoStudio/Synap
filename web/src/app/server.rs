use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::error::ServerFnError;
#[cfg(feature = "ssr")]
use synap_core::PeerDTO;
use synap_core::{
    dto::SyncSessionRecordDTO, LocalIdentityDTO, NoteDTO, PeerTrustStatusDTO, TimelineNotesPageDTO,
};

#[cfg(feature = "ssr")]
use axum::{
    body::Body,
    extract::Multipart,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
    Extension,
};
#[cfg(feature = "ssr")]
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
#[cfg(feature = "ssr")]
use corenet::{
    spawn_incoming_loop, IncomingLoopHandle, ListenConfig, ListenerState, NetError, TcpNetRuntime,
};
#[cfg(feature = "ssr")]
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[cfg(feature = "ssr")]
use synap_core::{ServiceError, SynapService, TimelineDirection};
#[cfg(feature = "ssr")]
use tokio::task::spawn_blocking;

#[cfg_attr(not(feature = "ssr"), allow(dead_code))]
const NOTE_LIMIT: usize = 50;
#[cfg_attr(not(feature = "ssr"), allow(dead_code))]
const TAG_SUGGESTION_LIMIT: usize = 6;
#[cfg(feature = "ssr")]
const DEFAULT_SYNC_PORT: u16 = 45_172;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebListenerStateDTO {
    pub protocol: String,
    pub backend: String,
    pub is_listening: bool,
    pub listen_port: Option<u16>,
    pub local_addresses: Vec<String>,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebPeerDTO {
    pub id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub display_public_key_base64: String,
    pub kaomoji_fingerprint: String,
    pub note: Option<String>,
    pub status: PeerTrustStatusDTO,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebSyncOverviewDTO {
    pub listener: WebListenerStateDTO,
    pub local_identity: LocalIdentityDTO,
    pub peers: Vec<WebPeerDTO>,
    pub recent_sync_sessions: Vec<SyncSessionRecordDTO>,
}

#[cfg(feature = "ssr")]
#[derive(Default)]
pub struct SyncRuntimeState {
    pub listener_handle: Mutex<Option<IncomingLoopHandle>>,
}

#[cfg(feature = "ssr")]
pub struct AppState {
    pub service: Mutex<Option<SynapService>>,
    pub db_path: PathBuf,
    pub sync_runtime: SyncRuntimeState,
}

#[cfg(feature = "ssr")]
pub type SharedService = Arc<AppState>;

#[cfg(not(feature = "ssr"))]
pub type SharedService = ();

#[cfg(feature = "ssr")]
async fn with_service<T, F>(f: F) -> Result<T, ServerFnError>
where
    T: Send + 'static,
    F: FnOnce(&SynapService) -> Result<T, ServiceError> + Send + 'static,
{
    let service = use_context::<SharedService>()
        .ok_or_else(|| ServerFnError::new("Synap service is not available"))?;
    spawn_blocking(move || -> Result<T, ServerFnError> {
        let guard = service
            .service
            .lock()
            .map_err(|_| ServerFnError::new("Synap service lock poisoned"))?;
        let inner = guard
            .as_ref()
            .ok_or_else(|| ServerFnError::new("Synap service is restarting"))?;

        f(inner).map_err(|error| ServerFnError::new(error.to_string()))
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

#[cfg(feature = "ssr")]
fn with_app_state() -> Result<SharedService, ServerFnError> {
    use_context::<SharedService>()
        .ok_or_else(|| ServerFnError::new("Synap service is not available"))
}

#[cfg(feature = "ssr")]
fn map_listener_state(state: ListenerState) -> WebListenerStateDTO {
    WebListenerStateDTO {
        protocol: state.protocol,
        backend: state.backend,
        is_listening: state.is_listening,
        listen_port: state.listen_port,
        local_addresses: state.local_addresses,
        status: state.status,
        error_message: state.error_message,
    }
}

#[cfg(feature = "ssr")]
fn map_peer(peer: PeerDTO) -> WebPeerDTO {
    WebPeerDTO {
        id: peer.id,
        algorithm: peer.algorithm,
        display_public_key_base64: BASE64_STANDARD.encode(&peer.public_key),
        public_key: peer.public_key,
        kaomoji_fingerprint: peer.kaomoji_fingerprint,
        note: peer.note,
        status: peer.status,
    }
}

#[cfg(feature = "ssr")]
fn listener_state_or_default(state: &SharedService) -> Result<WebListenerStateDTO, ServerFnError> {
    let guard = state
        .sync_runtime
        .listener_handle
        .lock()
        .map_err(|_| ServerFnError::new("sync listener lock poisoned"))?;
    Ok(guard
        .as_ref()
        .map(|handle| map_listener_state(handle.state()))
        .unwrap_or_else(|| map_listener_state(ListenerState::default())))
}

#[cfg(feature = "ssr")]
fn start_sync_listener(
    state: SharedService,
    preferred_port: Option<u16>,
) -> Result<WebListenerStateDTO, String> {
    let mut guard = state
        .sync_runtime
        .listener_handle
        .lock()
        .map_err(|_| "sync listener lock poisoned".to_string())?;

    if let Some(handle) = guard.as_ref() {
        let current = handle.state();
        if current.is_listening {
            return Ok(map_listener_state(current));
        }
    }

    let runtime = TcpNetRuntime;
    let listener = match runtime.listen(ListenConfig {
        port: preferred_port,
    }) {
        Ok(listener) => listener,
        Err(primary_error) if preferred_port.is_some() => runtime
            .listen(ListenConfig { port: None })
            .map_err(|fallback_error| {
                format!(
                    "failed to bind preferred port: {primary_error}; fallback failed: {fallback_error}"
                )
            })?,
        Err(error) => return Err(error.to_string()),
    };

    let app_state = Arc::clone(&state);
    let handle = spawn_incoming_loop(listener, move |incoming| match incoming {
        Ok(connection) => {
            let guard = match app_state.service.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    eprintln!("sync inbound failed: service lock poisoned");
                    return;
                }
            };
            let Some(service) = guard.as_ref() else {
                eprintln!("sync inbound skipped: service is restarting");
                return;
            };
            if let Err(error) = service.listen_sync(connection.channel) {
                eprintln!("sync inbound failed: {error}");
            }
        }
        Err(NetError::ListenerStopped) => {}
        Err(error) => {
            eprintln!("sync accept loop error: {error}");
        }
    });

    let current_state = map_listener_state(handle.state());
    *guard = Some(handle);
    Ok(current_state)
}

#[cfg(feature = "ssr")]
pub fn ensure_sync_listener_started(state: SharedService) -> Result<(), String> {
    let configured_port = std::env::var("SYNAP_SYNC_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .or(Some(DEFAULT_SYNC_PORT));
    start_sync_listener(state, configured_port).map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn export_db_handler(
    Extension(state): Extension<SharedService>,
) -> Result<Response, StatusCode> {
    let db_path = state.db_path.clone();
    let file_name = db_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("synap-web.redb")
        .to_string();

    let bytes = spawn_blocking(move || -> Result<Vec<u8>, StatusCode> {
        let guard = state
            .service
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if guard.is_none() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        std::fs::read(&db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}

#[cfg(feature = "ssr")]
pub async fn import_db_handler(
    Extension(state): Extension<SharedService>,
    mut multipart: Multipart,
) -> Response {
    let mut uploaded = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("database") {
            match field.bytes().await {
                Ok(bytes) if !bytes.is_empty() => {
                    uploaded = Some(bytes.to_vec());
                    break;
                }
                _ => return Redirect::to("/").into_response(),
            }
        }
    }

    let Some(bytes) = uploaded else {
        return Redirect::to("/").into_response();
    };

    let db_path = state.db_path.clone();
    let result = spawn_blocking(move || -> Result<(), String> {
        let parent_dir = db_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let temp_path = parent_dir.join("synap-web.import.redb");

        std::fs::write(&temp_path, &bytes).map_err(|error| error.to_string())?;

        let temp_path_string = temp_path.to_string_lossy().into_owned();
        SynapService::new(Some(temp_path_string.clone())).map_err(|error| error.to_string())?;

        let db_path_string = db_path.to_string_lossy().into_owned();
        let replacement =
            SynapService::new(Some(temp_path_string.clone())).map_err(|error| error.to_string())?;

        let mut guard = state
            .service
            .lock()
            .map_err(|_| "service lock poisoned".to_string())?;
        let old = guard.take();
        drop(old);

        if db_path.exists() {
            std::fs::remove_file(&db_path).map_err(|error| error.to_string())?;
        }
        std::fs::rename(&temp_path, &db_path).map_err(|error| error.to_string())?;

        drop(replacement);
        let live_service =
            SynapService::new(Some(db_path_string)).map_err(|error| error.to_string())?;
        *guard = Some(live_service);

        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => Redirect::to("/").into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to import database",
        )
            .into_response(),
    }
}

#[server]
pub async fn get_sync_overview_server() -> Result<WebSyncOverviewDTO, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state = with_app_state()?;
        let _ = start_sync_listener(state.clone(), Some(DEFAULT_SYNC_PORT));

        let listener = listener_state_or_default(&state)?;
        let local_identity = with_service(|service| service.get_local_identity()).await?;
        let peers = with_service(|service| service.get_peers()).await?;
        let recent_sync_sessions =
            with_service(|service| service.get_recent_sync_sessions(Some(10))).await?;

        Ok(WebSyncOverviewDTO {
            listener,
            local_identity,
            peers: peers.into_iter().map(map_peer).collect(),
            recent_sync_sessions,
        })
    }

    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("SSR is not enabled"))
    }
}

#[server]
pub async fn ensure_sync_listener_server() -> Result<WebListenerStateDTO, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state = with_app_state()?;
        start_sync_listener(state, Some(DEFAULT_SYNC_PORT)).map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("SSR is not enabled"))
    }
}

#[server]
pub async fn approve_peer_server(
    peer_id: String,
    note: Option<String>,
) -> Result<WebPeerDTO, ServerFnError> {
    let peer_id_for_note = peer_id.clone();
    let normalized_note = note.and_then(|value| {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    with_service(move |service| {
        if normalized_note.is_some() {
            let _ = service.update_peer_note(&peer_id_for_note, normalized_note.clone())?;
        }
        let peer = service.set_peer_status(&peer_id, PeerTrustStatusDTO::Trusted)?;
        Ok(map_peer(peer))
    })
    .await
}

#[server]
pub async fn update_peer_note_server(
    peer_id: String,
    note: Option<String>,
) -> Result<WebPeerDTO, ServerFnError> {
    let normalized_note = note.and_then(|value| {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    with_service(move |service| {
        service
            .update_peer_note(&peer_id, normalized_note)
            .map(map_peer)
    })
    .await
}

#[server]
pub async fn set_peer_status_server(
    peer_id: String,
    status: PeerTrustStatusDTO,
) -> Result<WebPeerDTO, ServerFnError> {
    with_service(move |service| service.set_peer_status(&peer_id, status).map(map_peer)).await
}

#[server]
pub async fn delete_peer_server(peer_id: String) -> Result<(), ServerFnError> {
    with_service(move |service| service.delete_peer(&peer_id)).await
}

#[server]
pub async fn list_notes_page(
    query: String,
    cursor: Option<String>,
) -> Result<TimelineNotesPageDTO, ServerFnError> {
    let query = query.trim().to_string();
    with_service(move |service| {
        if query.is_empty() {
            service.get_recent_notes_page(
                cursor.as_deref(),
                TimelineDirection::Older,
                Some(NOTE_LIMIT),
            )
        } else {
            Ok(TimelineNotesPageDTO {
                notes: service.search(&query, NOTE_LIMIT)?,
                next_cursor: None,
            })
        }
    })
    .await
}

#[server]
pub async fn create_note_server(
    content: String,
    #[server(default)] tags: Vec<String>,
) -> Result<NoteDTO, ServerFnError> {
    with_service(move |service| service.create_note(content, tags)).await
}

#[server]
pub async fn edit_note_server(
    note_id: String,
    content: String,
    #[server(default)] tags: Vec<String>,
) -> Result<NoteDTO, ServerFnError> {
    with_service(move |service| service.edit_note(&note_id, content, tags)).await
}

#[server]
pub async fn delete_note_server(note_id: String) -> Result<(), ServerFnError> {
    with_service(move |service| service.delete_note(&note_id)).await
}

#[server]
pub async fn recommend_tags_server(content: String) -> Result<Vec<String>, ServerFnError> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Ok(Vec::new());
    }

    with_service(move |service| service.recommend_tag(&content, TAG_SUGGESTION_LIMIT)).await
}
