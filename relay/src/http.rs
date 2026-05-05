use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::app::AppState;
use crate::error::AppError;
use crate::redis::{LeasedEnvelope, RelayStatusSnapshot, StoredEnvelope};
use synap_core::crypto::{SEALED_ENVELOPE_MAGIC, inspect_verified};

const MAILBOX_ROUTE: &str = "/v1/mailboxes/{mailbox_public_key}";
const ACK_ROUTE: &str = "/v1/mailboxes/{mailbox_public_key}/acks";
const STATUS_ROUTE: &str = "/status";
const AUTH_TIMESTAMP_HEADER: &str = "x-synap-timestamp";
const AUTH_SIGNATURE_HEADER: &str = "x-synap-signature";
const MESSAGE_SENDER_HEADER: &str = "x-synap-sender-ed25519";
const LEASE_ID_HEADER: &str = "x-synap-lease-id";
const LEASED_UNTIL_HEADER: &str = "x-synap-leased-until";

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route(STATUS_ROUTE, get(status))
        .route(MAILBOX_ROUTE, post(post_mailbox).get(get_mailbox))
        .route(ACK_ROUTE, post(post_ack))
        .with_state(state)
}

async fn root(State(state): State<AppState>) -> Json<ServiceInfo> {
    Json(ServiceInfo {
        service: "synap-relay",
        server_name: state.server_name().to_owned(),
        embedded_redis: state
            .embedded_redis()
            .map(|handle| handle.listen_addr().to_string()),
    })
}

async fn healthz(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    let redis = state.redis_runtime();
    let status = redis.health().await.map_err(|error| {
        error!(error = %error, "redis health probe failed");
        AppError::service_unavailable(format!("redis probe failed: {error}"))
    })?;

    Ok(Json(HealthResponse {
        status: "ok",
        server_name: state.server_name().to_owned(),
        redis: RedisHealth {
            url: redis.url().to_owned(),
            status: status.status,
            detail: status.detail,
            mode: redis.mode_label().to_owned(),
        },
    }))
}

async fn readyz(State(state): State<AppState>) -> Result<Json<ReadyResponse>, AppError> {
    let redis = state.redis_runtime();
    let status = redis.health().await.map_err(|error| {
        error!(error = %error, "redis readiness probe failed");
        AppError::service_unavailable(format!("redis not ready: {error}"))
    })?;

    Ok(Json(ReadyResponse {
        ready: true,
        redis_status: status.status,
    }))
}

async fn status(State(state): State<AppState>) -> Result<Json<StatusResponse>, AppError> {
    let snapshot = state.redis_runtime().status_snapshot().await.map_err(|error| {
        error!(error = %error, "failed to read relay status snapshot");
        AppError::service_unavailable(format!("failed to load relay status: {error}"))
    })?;

    Ok(Json(StatusResponse {
        service: "synap-relay",
        server_name: state.server_name().to_owned(),
        storage: StorageStatus {
            redis_url: state.redis_runtime().url().to_owned(),
            redis_mode: state.redis_runtime().mode_label().to_owned(),
        },
        metrics: snapshot.into(),
    }))
}

async fn post_mailbox(
    State(state): State<AppState>,
    Path(mailbox_public_key): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    let mailbox_public_key = normalize_mailbox_public_key_hex(&mailbox_public_key)?;
    if body.is_empty() {
        return Err(AppError::bad_request("request body must not be empty"));
    }
    if !body.starts_with(&SEALED_ENVELOPE_MAGIC) {
        return Err(AppError::bad_request("request body is not a sealed envelope"));
    }

    let inspected = inspect_verified(&body)
        .map_err(|error| AppError::bad_request(format!("invalid sealed envelope: {error}")))?;
    let sender_public_key_hex = hex::encode(inspected.sender_signing_public_key);

    state
        .redis_runtime()
        .put_latest_slot(
            &mailbox_public_key,
            StoredEnvelope {
                sender_public_key_hex,
                body: body.to_vec(),
            },
        )
        .await
        .map_err(|error| {
            error!(error = %error, mailbox = %mailbox_public_key, "failed to store relay envelope");
            AppError::service_unavailable(format!("failed to store envelope: {error}"))
        })?;

    Ok(StatusCode::ACCEPTED)
}

async fn get_mailbox(
    State(state): State<AppState>,
    Path(mailbox_public_key): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let mailbox_public_key = normalize_mailbox_public_key_hex(&mailbox_public_key)?;
    verify_mailbox_request_auth(&mailbox_public_key, &headers, "GET")?;

    match state
        .redis_runtime()
        .lease_next_slot(&mailbox_public_key)
        .await
        .map_err(|error| {
            error!(error = %error, mailbox = %mailbox_public_key, "failed to lease relay envelope");
            AppError::service_unavailable(format!("failed to load envelope: {error}"))
        })? {
        Some(leased) => Ok(leased_response(leased)?),
        None => Err(AppError::not_found("mailbox is empty")),
    }
}

async fn post_ack(
    State(state): State<AppState>,
    Path(mailbox_public_key): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<AckRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mailbox_public_key = normalize_mailbox_public_key_hex(&mailbox_public_key)?;
    verify_mailbox_request_auth(&mailbox_public_key, &headers, "POST")?;
    let sender_public_key_hex = normalize_mailbox_public_key_hex(&payload.sender_ed25519)?;
    if payload.lease_id.is_empty() {
        return Err(AppError::bad_request("lease_id must not be empty"));
    }

    let acknowledged = state
        .redis_runtime()
        .ack_slot(&mailbox_public_key, &sender_public_key_hex, &payload.lease_id)
        .await
        .map_err(|error| {
            error!(error = %error, mailbox = %mailbox_public_key, sender = %sender_public_key_hex, "failed to ack relay envelope");
            AppError::service_unavailable(format!("failed to ack envelope: {error}"))
        })?;

    if acknowledged {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("lease not found"))
    }
}

fn leased_response(leased: LeasedEnvelope) -> Result<impl IntoResponse, AppError> {
    let sender = HeaderValue::from_str(&leased.sender_public_key_hex)
        .map_err(|_| AppError::internal("invalid sender header value"))?;
    let lease_id = HeaderValue::from_str(&leased.lease_id)
        .map_err(|_| AppError::internal("invalid lease id header value"))?;
    let leased_until = HeaderValue::from_str(&leased.leased_until_ms.to_string())
        .map_err(|_| AppError::internal("invalid leased-until header value"))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream")),
            (HeaderName::from_static(MESSAGE_SENDER_HEADER), sender),
            (HeaderName::from_static(LEASE_ID_HEADER), lease_id),
            (HeaderName::from_static(LEASED_UNTIL_HEADER), leased_until),
        ],
        leased.body,
    ))
}

fn normalize_mailbox_public_key_hex(value: &str) -> Result<String, AppError> {
    let decoded = Vec::<u8>::from_hex(value)
        .map_err(|_| AppError::bad_request("mailbox public key must be lowercase or uppercase hex"))?;
    if decoded.len() != 32 {
        return Err(AppError::bad_request(
            "mailbox public key must decode to 32 bytes",
        ));
    }
    Ok(hex::encode(decoded))
}

fn header_string(headers: &HeaderMap, name: &'static str) -> Result<String, AppError> {
    let value = headers
        .get(name)
        .ok_or_else(|| AppError::unauthorized(format!("missing required header: {name}")))?;
    let value = value
        .to_str()
        .map_err(|_| AppError::unauthorized(format!("header {name} must be valid ASCII")))?;
    if value.is_empty() {
        return Err(AppError::unauthorized(format!("header {name} must not be empty")));
    }
    Ok(value.to_owned())
}

fn parse_signature_header(headers: &HeaderMap) -> Result<[u8; 64], AppError> {
    let signature_hex = header_string(headers, AUTH_SIGNATURE_HEADER)?;
    let bytes = Vec::<u8>::from_hex(&signature_hex)
        .map_err(|_| AppError::unauthorized("signature header must be hex"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| AppError::unauthorized("signature must decode to 64 bytes"))
}

fn verify_mailbox_request_auth(
    mailbox_public_key_hex: &str,
    headers: &HeaderMap,
    method: &str,
) -> Result<(), AppError> {
    let timestamp = header_string(headers, AUTH_TIMESTAMP_HEADER)?;
    let signature = parse_signature_header(headers)?;
    verify_mailbox_request_signature(mailbox_public_key_hex, &timestamp, &signature, method)
}

fn verify_mailbox_request_signature(
    mailbox_public_key_hex: &str,
    timestamp: &str,
    signature: &[u8; 64],
    method: &str,
) -> Result<(), AppError> {
    let public_key_bytes_vec = Vec::<u8>::from_hex(mailbox_public_key_hex)
        .map_err(|_| AppError::bad_request("mailbox public key must be hex"))?;
    let public_key_bytes: [u8; 32] = public_key_bytes_vec
        .as_slice()
        .try_into()
        .map_err(|_| AppError::bad_request("mailbox public key must decode to 32 bytes"))?;
    let public_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| AppError::bad_request("mailbox public key is not a valid Ed25519 key"))?;
    let signature = Signature::from_bytes(signature);
    let payload = auth_payload(mailbox_public_key_hex, timestamp, method);
    public_key
        .verify(payload.as_bytes(), &signature)
        .map_err(|_| AppError::unauthorized("invalid mailbox signature"))?;
    Ok(())
}

fn auth_payload(mailbox_public_key_hex: &str, timestamp: &str, method: &str) -> String {
    format!("{method}\n/v1/mailboxes/{mailbox_public_key_hex}\n{timestamp}")
}

#[derive(Serialize)]
struct ServiceInfo {
    service: &'static str,
    server_name: String,
    embedded_redis: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    server_name: String,
    redis: RedisHealth,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    redis_status: &'static str,
}

#[derive(Serialize)]
struct RedisHealth {
    url: String,
    mode: String,
    status: &'static str,
    detail: String,
}

#[derive(Serialize)]
struct StatusResponse {
    service: &'static str,
    server_name: String,
    storage: StorageStatus,
    metrics: RelayMetrics,
}

#[derive(Serialize)]
struct StorageStatus {
    redis_url: String,
    redis_mode: String,
}

#[derive(Serialize)]
struct RelayMetrics {
    mailbox_count: usize,
    buffered_slot_count: usize,
    leased_slot_count: usize,
    total_delivered_count: u64,
    total_post_count: u64,
    total_ack_count: u64,
    total_expired_count: u64,
    total_replaced_count: u64,
    total_lease_grant_count: u64,
    total_lease_expire_count: u64,
    total_rejected_ack_count: u64,
    oldest_slot_age_ms: Option<u64>,
    newest_slot_age_ms: Option<u64>,
}

impl From<RelayStatusSnapshot> for RelayMetrics {
    fn from(value: RelayStatusSnapshot) -> Self {
        Self {
            mailbox_count: value.mailbox_count,
            buffered_slot_count: value.total_buffered_slots,
            leased_slot_count: value.leased_slots,
            total_delivered_count: value.total_delivered_count,
            total_post_count: value.total_post_count,
            total_ack_count: value.total_ack_count,
            total_expired_count: value.total_expired_count,
            total_replaced_count: value.total_replaced_count,
            total_lease_grant_count: value.total_lease_grant_count,
            total_lease_expire_count: value.total_lease_expire_count,
            total_rejected_ack_count: value.total_rejected_ack_count,
            oldest_slot_age_ms: value.oldest_slot_age_ms,
            newest_slot_age_ms: value.newest_slot_age_ms,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AckRequest {
    sender_ed25519: String,
    lease_id: String,
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::Value;
    use tower::ServiceExt;

    use super::{
        ACK_ROUTE, AUTH_SIGNATURE_HEADER, AUTH_TIMESTAMP_HEADER, LEASE_ID_HEADER,
        MESSAGE_SENDER_HEADER, auth_payload, build_router,
    };
    use crate::{
        app::{AppState, AppStateParts},
        embedded_redis::EmbeddedRedisHandle,
        redis::RedisRuntime,
    };

    #[tokio::test]
    async fn post_get_ack_round_trip_works() -> anyhow::Result<()> {
        let (router, _handle) = test_router().await?;
        let recipient_key = test_signing_key([7; 32]);
        let sender_key = test_signing_key([8; 32]);
        let mailbox = hex::encode(recipient_key.verifying_key().to_bytes());
        let envelope = test_envelope(&sender_key);

        let response = router
            .clone()
            .oneshot(
                Request::post(format!("/v1/mailboxes/{mailbox}"))
                    .body(Body::from(envelope.clone()))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = router
            .clone()
            .oneshot(signed_mailbox_request("GET", &recipient_key, &mailbox, "1711111111", None)?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers().clone();
        let sender_header = headers.get(MESSAGE_SENDER_HEADER).unwrap().to_str()?;
        let lease_id = headers.get(LEASE_ID_HEADER).unwrap().to_str()?.to_owned();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        assert_eq!(sender_header, hex::encode(sender_key.verifying_key().to_bytes()));
        assert_eq!(body.as_ref(), envelope.as_slice());

        let ack_body = serde_json::to_vec(&serde_json::json!({
            "sender_ed25519": sender_header,
            "lease_id": lease_id,
        }))?;
        let response = router
            .clone()
            .oneshot(signed_mailbox_request(
                "POST",
                &recipient_key,
                &mailbox,
                "1711111112",
                Some((ACK_ROUTE.replace("{mailbox_public_key}", &mailbox), ack_body)),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = router
            .oneshot(signed_mailbox_request("GET", &recipient_key, &mailbox, "1711111113", None)?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn post_rejects_invalid_envelope() -> anyhow::Result<()> {
        let (router, _handle) = test_router().await?;
        let recipient_key = test_signing_key([11; 32]);
        let mailbox = hex::encode(recipient_key.verifying_key().to_bytes());

        let response = router
            .oneshot(
                Request::post(format!("/v1/mailboxes/{mailbox}"))
                    .body(Body::from(b"not-envelope".to_vec()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn get_rejects_invalid_signature() -> anyhow::Result<()> {
        let (router, _handle) = test_router().await?;
        let recipient_key = test_signing_key([13; 32]);
        let other_key = test_signing_key([14; 32]);
        let mailbox = hex::encode(recipient_key.verifying_key().to_bytes());

        let response = router
            .oneshot(signed_mailbox_request("GET", &other_key, &mailbox, "1711111111", None)?)
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn status_route_reports_buffered_and_delivered_metrics() -> anyhow::Result<()> {
        let (router, _handle) = test_router().await?;
        let recipient_key = test_signing_key([21; 32]);
        let sender_key = test_signing_key([22; 32]);
        let mailbox = hex::encode(recipient_key.verifying_key().to_bytes());
        let envelope = test_envelope(&sender_key);

        let response = router
            .clone()
            .oneshot(
                Request::post(format!("/v1/mailboxes/{mailbox}"))
                    .body(Body::from(envelope))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = router
            .clone()
            .oneshot(Request::get("/status").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let json: Value = serde_json::from_slice(&body)?;
        assert_eq!(json["metrics"]["buffered_slot_count"], 1);
        assert_eq!(json["metrics"]["total_post_count"], 1);

        Ok(())
    }

    async fn test_router() -> anyhow::Result<(axum::Router, EmbeddedRedisHandle)> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let redis_runtime = RedisRuntime::new(format!("redis://{}/", handle.listen_addr()))?;
        let state = AppState::from_parts(AppStateParts {
            server_name: "test-relay".to_owned(),
            redis_runtime,
            embedded_redis: None,
        });
        Ok((build_router(state), handle))
    }

    fn signed_mailbox_request(
        method: &str,
        signing_key: &SigningKey,
        mailbox_public_key_hex: &str,
        timestamp: &str,
        body: Option<(String, Vec<u8>)>,
    ) -> anyhow::Result<Request<Body>> {
        let path = body
            .as_ref()
            .map(|(path, _)| path.clone())
            .unwrap_or_else(|| format!("/v1/mailboxes/{mailbox_public_key_hex}"));
        let payload = auth_payload(mailbox_public_key_hex, timestamp, method);
        let signature = signing_key.sign(payload.as_bytes()).to_bytes();

        let builder = Request::builder()
            .method(method)
            .uri(path)
            .header(AUTH_TIMESTAMP_HEADER, timestamp)
            .header(AUTH_SIGNATURE_HEADER, hex::encode(signature));

        match body {
            Some((_, body)) => Ok(builder
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))?),
            None => Ok(builder.body(Body::empty())?),
        }
    }

    fn test_signing_key(secret: [u8; 32]) -> SigningKey {
        SigningKey::from_bytes(&secret)
    }

    fn test_envelope(signing_key: &SigningKey) -> Vec<u8> {
        let sender = signing_key.verifying_key().to_bytes();
        let recipient_identity_public_key = [33u8; 32];
        let ephemeral_identity_public_key = [44u8; 32];
        let sealed_payload = b"opaque-payload";

        let mut signature_payload = Vec::new();
        signature_payload.extend_from_slice(b"synap.crypto.sealed-envelope.signature.v1");
        signature_payload.push(1);
        signature_payload.extend_from_slice(&sender);
        signature_payload.extend_from_slice(&recipient_identity_public_key);
        signature_payload.extend_from_slice(&ephemeral_identity_public_key);
        signature_payload.extend_from_slice(sealed_payload);

        let signature = signing_key.sign(&signature_payload).to_bytes().to_vec();
        let wire = TestSealedEnvelopeWire {
            version: 1,
            sender_signing_public_key: sender,
            recipient_identity_public_key,
            ephemeral_identity_public_key,
            sealed_payload: sealed_payload.to_vec(),
            signature,
        };

        let mut encoded = Vec::from(*b"SKE!");
        encoded.extend_from_slice(&postcard::to_allocvec(&wire).unwrap());
        encoded
    }

    #[derive(serde::Serialize)]
    struct TestSealedEnvelopeWire {
        version: u8,
        sender_signing_public_key: [u8; 32],
        recipient_identity_public_key: [u8; 32],
        ephemeral_identity_public_key: [u8; 32],
        sealed_payload: Vec<u8>,
        signature: Vec<u8>,
    }
}
