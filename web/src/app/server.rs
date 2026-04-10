use leptos::prelude::*;
use server_fn::error::ServerFnError;
use synap_core::{NoteDTO, TimelineNotesPageDTO};

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
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[cfg(feature = "ssr")]
use synap_core::{ServiceError, SynapService, TimelineDirection};
#[cfg(feature = "ssr")]
use tokio::task::spawn_blocking;

#[cfg(feature = "ssr")]
pub struct AppState {
    pub service: Mutex<Option<SynapService>>,
    pub db_path: PathBuf,
}

#[cfg(feature = "ssr")]
pub type SharedService = Arc<AppState>;

#[cfg(not(feature = "ssr"))]
pub type SharedService = ();

#[cfg_attr(not(feature = "ssr"), allow(dead_code))]
const NOTE_LIMIT: usize = 50;
#[cfg_attr(not(feature = "ssr"), allow(dead_code))]
const TAG_SUGGESTION_LIMIT: usize = 6;

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
