use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hex::FromHex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    crypto,
    error::ServiceError,
    models::sync_stats::{
        SyncSessionRole, SyncSessionStatus, SyncStatsRecord, SyncStatsWriter, SyncTransportKind,
    },
    service::SynapService,
};

const AUTH_TIMESTAMP_HEADER: &str = "x-synap-timestamp";
const AUTH_SIGNATURE_HEADER: &str = "x-synap-signature";
const MESSAGE_SENDER_HEADER: &str = "x-synap-sender-ed25519";
const LEASE_ID_HEADER: &str = "x-synap-lease-id";
const LEASED_UNTIL_HEADER: &str = "x-synap-leased-until";
const API_KEY_HEADER: &str = "x-api-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayLeasedEnvelope {
    pub sender_ed25519: [u8; 32],
    pub lease_id: String,
    pub leased_until_ms: u64,
    pub envelope_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayOpenedEnvelopeLease {
    pub sender_ed25519: [u8; 32],
    pub lease_id: String,
    pub leased_until_ms: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelaySyncEnvelope {
    pub inventory: super::RelayInventory,
    pub share: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayFetchStats {
    pub fetched_messages: u64,
    pub imported_messages: u64,
    pub dropped_untrusted_messages: u64,
    pub acked_messages: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayPushStats {
    pub trusted_peers: u64,
    pub posted_messages: u64,
    pub full_sync_messages: u64,
    pub incremental_sync_messages: u64,
}

pub struct RelayHttpService<'a> {
    core: &'a SynapService,
    base_url: String,
    timeout: Duration,
    api_key: Option<String>,
}

#[derive(Debug, Error)]
pub enum RelayHttpError {
    #[error(transparent)]
    Service(#[from] ServiceError),

    #[error("local signing identity is missing")]
    MissingLocalSigningIdentity,

    #[error("relay request timed out: {0}")]
    Timeout(String),

    #[error("relay connection failed: {0}")]
    Connection(String),

    #[error("relay transport error: {0}")]
    Transport(String),

    #[error("relay rejected authentication: {code:?}: {message}")]
    Unauthorized {
        code: Option<String>,
        message: String,
    },

    #[error("relay service is unavailable: {code:?}: {message}")]
    ServiceUnavailable {
        code: Option<String>,
        message: String,
    },

    #[error("relay returned unexpected status {status}: {code:?}: {message}")]
    HttpStatus {
        status: u16,
        code: Option<String>,
        message: String,
    },

    #[error("relay response missing header: {0}")]
    MissingHeader(&'static str),

    #[error("relay response header {0} is invalid")]
    InvalidHeader(&'static str),

    #[error("relay returned malformed sender public key")]
    InvalidSenderPublicKey,

    #[error("relay ack lease was not found")]
    LeaseNotFound,

    #[error("sealed envelope error: {0}")]
    SealedEnvelope(String),

    #[error("relay envelope payload is invalid: {0}")]
    InvalidPayload(String),
}

impl<'a> RelayHttpService<'a> {
    pub fn new(core: &'a SynapService, base_url: impl Into<String>) -> Self {
        Self {
            core,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            timeout: Duration::from_secs(10),
            api_key: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn post_envelope(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
        envelope_bytes: &[u8],
    ) -> Result<(), RelayHttpError> {
        let url = self.mailbox_url(recipient_mailbox_ed25519);
        let request = ureq::post(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header("content-type", "application/octet-stream");
        let request = match &self.api_key {
            Some(api_key) => request.header(API_KEY_HEADER, api_key),
            None => request,
        };
        let mut response = request.send(envelope_bytes).map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        if status == 202 {
            return Ok(());
        }

        Err(map_error_response(
            status,
            read_response_text(&mut response)?,
        ))
    }

    pub fn get_envelope(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
    ) -> Result<Option<RelayLeasedEnvelope>, RelayHttpError> {
        let timestamp = now_ms().to_string();
        let auth = self.mailbox_auth(recipient_mailbox_ed25519, "GET", &timestamp)?;
        let url = self.mailbox_url(recipient_mailbox_ed25519);

        let request = ureq::get(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header(AUTH_TIMESTAMP_HEADER, &timestamp)
            .header(AUTH_SIGNATURE_HEADER, &auth.signature_hex);
        let request = match &self.api_key {
            Some(api_key) => request.header(API_KEY_HEADER, api_key),
            None => request,
        };
        let mut response = request.call().map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        if status != 200 {
            let body = read_response_text(&mut response)?;
            if status == 204 || is_mailbox_empty(status, &body) {
                return Ok(None);
            }
            return Err(map_error_response(status, body));
        }

        let sender_hex = response
            .headers()
            .get(MESSAGE_SENDER_HEADER)
            .ok_or(RelayHttpError::MissingHeader(MESSAGE_SENDER_HEADER))?
            .to_str()
            .map_err(|_| RelayHttpError::InvalidHeader(MESSAGE_SENDER_HEADER))?;
        let sender_ed25519 = parse_public_key_hex(sender_hex)?;

        let lease_id = response
            .headers()
            .get(LEASE_ID_HEADER)
            .ok_or(RelayHttpError::MissingHeader(LEASE_ID_HEADER))?
            .to_str()
            .map_err(|_| RelayHttpError::InvalidHeader(LEASE_ID_HEADER))?
            .to_owned();

        let leased_until_ms = response
            .headers()
            .get(LEASED_UNTIL_HEADER)
            .ok_or(RelayHttpError::MissingHeader(LEASED_UNTIL_HEADER))?
            .to_str()
            .map_err(|_| RelayHttpError::InvalidHeader(LEASED_UNTIL_HEADER))?
            .parse::<u64>()
            .map_err(|_| RelayHttpError::InvalidHeader(LEASED_UNTIL_HEADER))?;

        let envelope_bytes = response.body_mut().read_to_vec().map_err(map_ureq_error)?;

        Ok(Some(RelayLeasedEnvelope {
            sender_ed25519,
            lease_id,
            leased_until_ms,
            envelope_bytes,
        }))
    }

    pub fn get_and_open_for_local_recipient(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
    ) -> Result<Option<RelayOpenedEnvelopeLease>, RelayHttpError> {
        let Some(leased) = self.get_envelope(recipient_mailbox_ed25519)? else {
            return Ok(None);
        };

        let opened = self
            .core
            .with_read(|tx, _reader| {
                let crypto_reader = crate::models::crypto::CryptoReader::new(tx)?;
                Ok(crypto::open_for_local_recipient(
                    &crypto_reader,
                    &leased.envelope_bytes,
                ))
            })
            .map_err(RelayHttpError::Service)?
            .map_err(|err| RelayHttpError::SealedEnvelope(err.to_string()))?;

        Ok(Some(RelayOpenedEnvelopeLease {
            sender_ed25519: opened.sender.public_key,
            lease_id: leased.lease_id,
            leased_until_ms: leased.leased_until_ms,
            payload: opened.payload,
        }))
    }

    pub fn ack_envelope(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
        sender_ed25519: [u8; 32],
        lease_id: &str,
    ) -> Result<(), RelayHttpError> {
        let timestamp = now_ms().to_string();
        let auth = self.mailbox_auth(recipient_mailbox_ed25519, "POST", &timestamp)?;
        let url = self.ack_url(recipient_mailbox_ed25519);
        let body = serde_json::to_string(&RelayAckRequest {
            sender_ed25519: hex::encode(sender_ed25519),
            lease_id: lease_id.to_owned(),
        })
        .map_err(|err| RelayHttpError::Transport(err.to_string()))?;

        let request = ureq::post(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header(AUTH_TIMESTAMP_HEADER, &timestamp)
            .header(AUTH_SIGNATURE_HEADER, &auth.signature_hex)
            .header("content-type", "application/json");
        let request = match &self.api_key {
            Some(api_key) => request.header(API_KEY_HEADER, api_key),
            None => request,
        };
        let mut response = request.send(body.as_str()).map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        match status {
            204 => Ok(()),
            404 => Err(RelayHttpError::LeaseNotFound),
            _ => {
                let body = read_response_text(&mut response)?;
                Err(map_error_response(status, body))
            }
        }
    }

    pub fn fetch_relay_updates(&self) -> Result<RelayFetchStats, RelayHttpError> {
        let recipient_mailbox_ed25519 = self.core.local_relay_mailbox_public_key()?;
        let mut stats = RelayFetchStats {
            fetched_messages: 0,
            imported_messages: 0,
            dropped_untrusted_messages: 0,
            acked_messages: 0,
        };

        loop {
            let Some(leased) = self.get_and_open_for_local_recipient(recipient_mailbox_ed25519)?
            else {
                return Ok(stats);
            };
            stats.fetched_messages += 1;

            if !self.is_trusted_sender(leased.sender_ed25519)? {
                self.ack_envelope(
                    recipient_mailbox_ed25519,
                    leased.sender_ed25519,
                    &leased.lease_id,
                )?;
                stats.dropped_untrusted_messages += 1;
                stats.acked_messages += 1;
                continue;
            }

            let envelope: RelaySyncEnvelope = postcard::from_bytes(&leased.payload)
                .map_err(|err| RelayHttpError::InvalidPayload(err.to_string()))?;
            envelope
                .inventory
                .validate()
                .map_err(|err| RelayHttpError::InvalidPayload(err.to_string()))?;

            let started_at_ms = now_ms();
            let share_bytes = envelope.share.len() as u64;
            self.core.import_share(&envelope.share)?;
            self.core.cache_relay_peer_inventory(
                &leased.sender_ed25519,
                envelope.inventory,
                now_ms(),
            )?;
            self.ack_envelope(
                recipient_mailbox_ed25519,
                leased.sender_ed25519,
                &leased.lease_id,
            )?;
            stats.imported_messages += 1;
            stats.acked_messages += 1;
            self.record_relay_sync_session(
                leased.sender_ed25519,
                SyncSessionRole::RelayFetch,
                SyncTransportKind::RelayFetch {
                    relay_url: self.base_url.clone(),
                },
                started_at_ms,
                RelaySyncRecordStats {
                    records_sent: 0,
                    records_received: 1,
                    records_applied: 1,
                    records_skipped: 0,
                    bytes_sent: 0,
                    bytes_received: share_bytes,
                },
            )?;
        }
    }

    pub fn push_relay_updates(&self) -> Result<RelayPushStats, RelayHttpError> {
        let trusted_peers = self.trusted_peer_public_keys()?;
        let local_inventory = self.core.build_relay_inventory()?;
        let mut stats = RelayPushStats {
            trusted_peers: trusted_peers.len() as u64,
            posted_messages: 0,
            full_sync_messages: 0,
            incremental_sync_messages: 0,
        };

        for peer_public_key in trusted_peers {
            let cached = self.core.get_relay_peer(&peer_public_key)?;
            let remote_inventory = cached
                .as_ref()
                .map(|record| record.cached_inventory.clone())
                .unwrap_or_else(empty_relay_inventory);
            let share = self
                .core
                .export_relay_share_for_inventory(&remote_inventory)?;
            let payload = RelaySyncEnvelope {
                inventory: local_inventory.clone(),
                share,
            };
            let encoded = postcard::to_allocvec(&payload)
                .map_err(|err| RelayHttpError::InvalidPayload(err.to_string()))?;
            let envelope = self
                .core
                .seal_relay_payload_for(peer_public_key, &encoded)?;
            let started_at_ms = now_ms();
            let envelope_bytes = envelope.len() as u64;
            self.post_envelope(peer_public_key, &envelope)?;

            stats.posted_messages += 1;
            if cached.is_some() {
                stats.incremental_sync_messages += 1;
            } else {
                stats.full_sync_messages += 1;
            }
            self.record_relay_sync_session(
                peer_public_key,
                SyncSessionRole::RelayPush,
                SyncTransportKind::RelayPush {
                    relay_url: self.base_url.clone(),
                },
                started_at_ms,
                RelaySyncRecordStats {
                    records_sent: 1,
                    records_received: 0,
                    records_applied: 0,
                    records_skipped: 0,
                    bytes_sent: envelope_bytes,
                    bytes_received: 0,
                },
            )?;
        }

        Ok(stats)
    }

    fn mailbox_url(&self, recipient_mailbox_ed25519: [u8; 32]) -> String {
        format!(
            "{}/v1/mailboxes/{}",
            self.base_url,
            hex::encode(recipient_mailbox_ed25519)
        )
    }

    fn ack_url(&self, recipient_mailbox_ed25519: [u8; 32]) -> String {
        format!("{}/acks", self.mailbox_url(recipient_mailbox_ed25519))
    }

    fn mailbox_auth(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
        method: &str,
        timestamp: &str,
    ) -> Result<RelayMailboxAuth, RelayHttpError> {
        let payload = mailbox_auth_payload(recipient_mailbox_ed25519, method, timestamp);
        let signature = self
            .core
            .sign_relay_mailbox_auth(payload.as_bytes())
            .map_err(|error| match error {
                ServiceError::Other(other)
                    if other
                        .to_string()
                        .contains("local signing identity is missing") =>
                {
                    RelayHttpError::MissingLocalSigningIdentity
                }
                other => RelayHttpError::Service(other),
            })?;

        Ok(RelayMailboxAuth {
            signature_hex: hex::encode(signature),
        })
    }

    fn is_trusted_sender(&self, sender_ed25519: [u8; 32]) -> Result<bool, RelayHttpError> {
        self.core
            .with_read(|tx, _reader| {
                let crypto_reader = crate::models::crypto::CryptoReader::new(tx)?;
                crypto::is_trusted_public_key(&crypto_reader, sender_ed25519).map_err(Into::into)
            })
            .map_err(RelayHttpError::Service)
    }

    fn trusted_peer_public_keys(&self) -> Result<Vec<[u8; 32]>, RelayHttpError> {
        self.core
            .with_read(|tx, _reader| {
                let crypto_reader = crate::models::crypto::CryptoReader::new(tx)?;
                crypto::list_trusted_public_keys(&crypto_reader)
                    .map(|records| {
                        records
                            .into_iter()
                            .map(|record| record.public_key)
                            .collect::<Vec<_>>()
                    })
                    .map_err(Into::into)
            })
            .map_err(RelayHttpError::Service)
    }

    fn record_relay_sync_session(
        &self,
        peer_public_key: [u8; 32],
        role: SyncSessionRole,
        transport: SyncTransportKind,
        started_at_ms: u64,
        stats: RelaySyncRecordStats,
    ) -> Result<(), RelayHttpError> {
        let finished_at_ms = now_ms();
        let record = SyncStatsRecord {
            id: uuid::Uuid::now_v7(),
            role,
            status: SyncSessionStatus::Completed,
            peer_public_key,
            transport,
            started_at_ms,
            finished_at_ms,
            records_sent: stats.records_sent,
            records_received: stats.records_received,
            records_applied: stats.records_applied,
            records_skipped: stats.records_skipped,
            bytes_sent: stats.bytes_sent,
            bytes_received: stats.bytes_received,
            duration_ms: finished_at_ms.saturating_sub(started_at_ms),
            error_message: None,
        };

        self.core
            .with_write(|tx| {
                let writer = SyncStatsWriter::new(tx);
                writer.put(&record)?;
                Ok(())
            })
            .map_err(RelayHttpError::Service)
    }
}

struct RelaySyncRecordStats {
    records_sent: u64,
    records_received: u64,
    records_applied: u64,
    records_skipped: u64,
    bytes_sent: u64,
    bytes_received: u64,
}

struct RelayMailboxAuth {
    signature_hex: String,
}

#[derive(Debug, Serialize)]
struct RelayAckRequest {
    sender_ed25519: String,
    lease_id: String,
}

#[derive(Debug, Deserialize)]
struct RelayErrorBody {
    #[serde(default)]
    code: Option<String>,
    #[serde(default, alias = "error")]
    message: String,
}

fn mailbox_auth_payload(
    recipient_mailbox_ed25519: [u8; 32],
    method: &str,
    timestamp: &str,
) -> String {
    format!(
        "{method}\n/v1/mailboxes/{}\n{timestamp}",
        hex::encode(recipient_mailbox_ed25519)
    )
}

fn parse_public_key_hex(value: &str) -> Result<[u8; 32], RelayHttpError> {
    let bytes = Vec::<u8>::from_hex(value).map_err(|_| RelayHttpError::InvalidSenderPublicKey)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| RelayHttpError::InvalidSenderPublicKey)
}

fn empty_relay_inventory() -> super::RelayInventory {
    super::RelayInventory {
        version: super::RelayInventory::VERSION,
        records: Vec::new(),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

fn read_response_text(
    response: &mut ureq::http::Response<ureq::Body>,
) -> Result<String, RelayHttpError> {
    response.body_mut().read_to_string().map_err(map_ureq_error)
}

fn parse_relay_error_body(body: &str) -> (Option<String>, String) {
    match serde_json::from_str::<RelayErrorBody>(body) {
        Ok(parsed) => (parsed.code, parsed.message),
        Err(_) => (None, body.to_owned()),
    }
}

fn is_mailbox_empty(status: u16, body: &str) -> bool {
    if status != 404 {
        return false;
    }
    let (code, message) = parse_relay_error_body(body);
    code.as_deref() == Some("mailbox_empty") || (code.is_none() && message.is_empty())
}

fn map_error_response(status: u16, body: String) -> RelayHttpError {
    let (code, message) = parse_relay_error_body(&body);
    match (status, code.as_deref()) {
        (401 | 403, _) => RelayHttpError::Unauthorized { code, message },
        (503, _) => RelayHttpError::ServiceUnavailable { code, message },
        (404, Some("lease_not_found")) => RelayHttpError::LeaseNotFound,
        _ => RelayHttpError::HttpStatus {
            status,
            code,
            message,
        },
    }
}

fn map_ureq_error(err: ureq::Error) -> RelayHttpError {
    match err {
        ureq::Error::StatusCode(status) => RelayHttpError::HttpStatus {
            status,
            code: None,
            message: String::new(),
        },
        ureq::Error::Timeout(error) => RelayHttpError::Timeout(error.to_string()),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            RelayHttpError::Connection(err.to_string())
        }
        ureq::Error::Io(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::AddrNotAvailable
                    | std::io::ErrorKind::AddrInUse
            ) =>
        {
            RelayHttpError::Connection(error.to_string())
        }
        ureq::Error::Io(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            RelayHttpError::Timeout(error.to_string())
        }
        other => RelayHttpError::Transport(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, sync::mpsc, thread};

    use crate::service::SynapService;

    use super::*;

    #[test]
    fn auth_payload_matches_relay_contract() {
        let recipient = [0x11; 32];

        let payload = mailbox_auth_payload(recipient, "GET", "1711111111");

        assert_eq!(
            payload,
            format!("GET\n/v1/mailboxes/{}\n1711111111", hex::encode(recipient))
        );
    }

    #[test]
    fn get_and_open_for_local_recipient_reads_headers_and_decrypts() {
        let service = SynapService::open_memory().unwrap();
        let recipient_mailbox = service.local_relay_mailbox_public_key().unwrap();
        let envelope = sender_test_envelope(recipient_mailbox);
        let body = envelope.clone();
        let sender_ed25519 = sender_signing_public_key_from_envelope(&envelope);

        let server = fake_server(move |stream| {
            read_http_request(&stream);
            let mut response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\n{}: {}\r\n{}: lease-1\r\n{}: 1711111111\r\ncontent-length: {}\r\n\r\n",
                MESSAGE_SENDER_HEADER,
                hex::encode(sender_ed25519),
                LEASE_ID_HEADER,
                LEASED_UNTIL_HEADER,
                body.len()
            )
            .into_bytes();
            response.extend_from_slice(&body);
            write_http_response(stream, &response);
        });

        let client = RelayHttpService::new(&service, server.base_url());
        let opened = client
            .get_and_open_for_local_recipient(recipient_mailbox)
            .unwrap()
            .unwrap();

        assert_eq!(opened.sender_ed25519, sender_ed25519);
        assert_eq!(opened.lease_id, "lease-1");
        assert_eq!(opened.payload, b"relay-payload");
    }

    #[test]
    fn get_envelope_returns_none_for_mailbox_empty_code() {
        let service = SynapService::open_memory().unwrap();
        let recipient_mailbox = service.local_relay_mailbox_public_key().unwrap();

        let server = fake_server(|stream| {
            read_http_request(&stream);
            write_json_response(
                stream,
                "404 Not Found",
                r#"{"code":"mailbox_empty","error":"mailbox is empty"}"#,
            );
        });

        let client = RelayHttpService::new(&service, server.base_url());
        let result = client.get_envelope(recipient_mailbox).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn post_envelope_maps_auth_error_code() {
        let sender = SynapService::open_memory().unwrap();
        let recipient = SynapService::open_memory().unwrap();
        let recipient_mailbox = recipient.local_relay_mailbox_public_key().unwrap();
        let envelope = sender
            .seal_relay_payload_for(recipient_mailbox, b"relay-payload")
            .unwrap();

        let server = fake_server(|stream| {
            read_http_request(&stream);
            write_json_response(
                stream,
                "401 Unauthorized",
                r#"{"code":"invalid_api_key","error":"invalid relay api key"}"#,
            );
        });

        let client = RelayHttpService::new(&sender, server.base_url()).with_api_key("wrong");
        let result = client.post_envelope(recipient_mailbox, &envelope);

        assert!(matches!(
            result,
            Err(RelayHttpError::Unauthorized {
                code: Some(code),
                message,
            }) if code == "invalid_api_key" && message == "invalid relay api key"
        ));
    }

    #[test]
    fn ack_envelope_returns_lease_not_found_for_404() {
        let service = SynapService::open_memory().unwrap();
        let recipient_mailbox = service.local_relay_mailbox_public_key().unwrap();

        let server = fake_server(|stream| {
            read_http_request(&stream);
            write_json_response(
                stream,
                "404 Not Found",
                r#"{"code":"lease_not_found","error":"lease not found"}"#,
            );
        });

        let client = RelayHttpService::new(&service, server.base_url());
        let result = client.ack_envelope(recipient_mailbox, [0x22; 32], "lease-1");

        assert!(matches!(result, Err(RelayHttpError::LeaseNotFound)));
    }

    #[test]
    fn post_envelope_sends_api_key_header_when_configured() {
        let sender = SynapService::open_memory().unwrap();
        let recipient = SynapService::open_memory().unwrap();
        let recipient_mailbox = recipient.local_relay_mailbox_public_key().unwrap();
        let envelope = sender
            .seal_relay_payload_for(recipient_mailbox, b"relay-payload")
            .unwrap();
        let expected_api_key = "relay-secret".to_owned();

        let server = fake_server(move |stream| {
            let request = read_http_request(&stream);
            assert!(request.contains(&format!("{API_KEY_HEADER}: {expected_api_key}\r\n")));
            write_http_response(
                stream,
                b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n",
            );
        });

        let client = RelayHttpService::new(&sender, server.base_url()).with_api_key("relay-secret");
        client.post_envelope(recipient_mailbox, &envelope).unwrap();
    }

    #[test]
    fn fetch_relay_updates_imports_trusted_sender_share_and_updates_inventory_cache() {
        let sender = SynapService::open_memory().unwrap();
        let recipient = SynapService::open_memory().unwrap();
        let sender_identity = sender.get_local_identity().unwrap();
        let sender_ed25519: [u8; 32] = sender_identity
            .signing
            .public_key
            .as_slice()
            .try_into()
            .unwrap();
        recipient
            .trust_peer(&sender_ed25519, Some("trusted-sender".into()))
            .unwrap();

        let note = sender
            .create_note("relay fetched note".to_owned(), vec!["relay".to_owned()])
            .unwrap();
        let sender_inventory = sender.build_relay_inventory().unwrap();
        let share = sender
            .export_relay_share_for_inventory(&empty_relay_inventory())
            .unwrap();
        let envelope_bytes = relay_sync_test_envelope(
            &sender,
            recipient.local_relay_mailbox_public_key().unwrap(),
            RelaySyncEnvelope {
                inventory: sender_inventory.clone(),
                share,
            },
        );
        let sender_header = hex::encode(sender_ed25519);

        let server = multi_request_server(vec![
            FakeResponse::mailbox_ok(sender_header.clone(), "lease-1", envelope_bytes),
            FakeResponse::ack_no_content(),
            FakeResponse::mailbox_empty(),
        ]);

        let client = RelayHttpService::new(&recipient, server.base_url());
        let stats = client.fetch_relay_updates().unwrap();

        assert_eq!(
            stats,
            RelayFetchStats {
                fetched_messages: 1,
                imported_messages: 1,
                dropped_untrusted_messages: 0,
                acked_messages: 1,
            }
        );

        let notes = recipient.get_recent_note(None, Some(20)).unwrap();
        assert!(notes.iter().any(|item| item.id == note.id));

        let cached = recipient.get_relay_peer(&sender_ed25519).unwrap().unwrap();
        assert_eq!(cached.peer_public_key, sender_ed25519);
        assert_eq!(cached.cached_inventory, sender_inventory);

        let peer_stats = recipient.get_peer_sync_stats(Some(10), Some(5)).unwrap();
        assert_eq!(peer_stats.len(), 1);
        assert_eq!(peer_stats[0].peer_public_key, sender_ed25519);
        assert_eq!(
            peer_stats[0].recent_sessions[0].role,
            crate::dto::SyncSessionRoleDTO::RelayFetch
        );
    }

    #[test]
    fn fetch_relay_updates_drops_and_acks_untrusted_sender_messages() {
        let sender = SynapService::open_memory().unwrap();
        let recipient = SynapService::open_memory().unwrap();
        let sender_identity = sender.get_local_identity().unwrap();
        let sender_ed25519: [u8; 32] = sender_identity
            .signing
            .public_key
            .as_slice()
            .try_into()
            .unwrap();

        let sender_inventory = sender.build_relay_inventory().unwrap();
        let share = sender
            .export_relay_share_for_inventory(&empty_relay_inventory())
            .unwrap();
        let envelope_bytes = relay_sync_test_envelope(
            &sender,
            recipient.local_relay_mailbox_public_key().unwrap(),
            RelaySyncEnvelope {
                inventory: sender_inventory,
                share,
            },
        );
        let sender_header = hex::encode(sender_ed25519);

        let server = multi_request_server(vec![
            FakeResponse::mailbox_ok(sender_header.clone(), "lease-1", envelope_bytes),
            FakeResponse::ack_no_content(),
            FakeResponse::mailbox_empty(),
        ]);

        let client = RelayHttpService::new(&recipient, server.base_url());
        let stats = client.fetch_relay_updates().unwrap();

        assert_eq!(
            stats,
            RelayFetchStats {
                fetched_messages: 1,
                imported_messages: 0,
                dropped_untrusted_messages: 1,
                acked_messages: 1,
            }
        );
        assert!(recipient.get_relay_peer(&sender_ed25519).unwrap().is_none());
    }

    #[test]
    fn push_relay_updates_posts_full_for_unknown_peer_and_incremental_for_cached_peer() {
        let service = SynapService::open_memory().unwrap();
        let peer_without_cache = SynapService::open_memory().unwrap();
        let peer_with_cache = SynapService::open_memory().unwrap();

        let peer_without_cache_key = peer_without_cache.local_relay_mailbox_public_key().unwrap();
        let peer_with_cache_key = peer_with_cache.local_relay_mailbox_public_key().unwrap();
        service
            .trust_peer(&peer_without_cache_key, Some("peer-without-cache".into()))
            .unwrap();
        service
            .trust_peer(&peer_with_cache_key, Some("peer-with-cache".into()))
            .unwrap();

        let cached_inventory = peer_with_cache.build_relay_inventory().unwrap();
        service
            .cache_relay_peer_inventory(&peer_with_cache_key, cached_inventory, 111)
            .unwrap();

        let note = service
            .create_note("push relay note".to_owned(), vec!["relay".to_owned()])
            .unwrap();
        let note_id: uuid::Uuid = note.id.parse().unwrap();

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let server = capture_post_bodies_server(
            vec![FakeResponse::accepted(), FakeResponse::accepted()],
            captured.clone(),
        );

        let client = RelayHttpService::new(&service, server.base_url());
        let stats = client.push_relay_updates().unwrap();

        assert_eq!(
            stats,
            RelayPushStats {
                trusted_peers: 2,
                posted_messages: 2,
                full_sync_messages: 1,
                incremental_sync_messages: 1,
            }
        );

        let captured = captured.lock().unwrap().clone();
        assert_eq!(captured.len(), 2);

        let envelope_a =
            open_test_payload_for_any(&peer_without_cache, &peer_with_cache, &captured[0]);
        let envelope_b =
            open_test_payload_for_any(&peer_without_cache, &peer_with_cache, &captured[1]);
        let envelopes = [envelope_a, envelope_b];

        assert!(envelopes.iter().all(|envelope| envelope
            .inventory
            .records
            .iter()
            .any(|record| record.root_note_id.to_string() == note.id)));
        assert!(envelopes.iter().all(|envelope| !envelope.share.is_empty()));
        assert!(envelopes.iter().any(|envelope| envelope
            .inventory
            .records
            .iter()
            .any(|record| record.root_note_id == note_id)));

        let peer_stats = service.get_peer_sync_stats(Some(10), Some(5)).unwrap();
        assert_eq!(peer_stats.len(), 2);
        assert!(peer_stats.iter().all(|group| group
            .recent_sessions
            .iter()
            .all(|session| session.role == crate::dto::SyncSessionRoleDTO::RelayPush)));
    }

    fn sender_test_envelope(recipient_mailbox_public_key: [u8; 32]) -> Vec<u8> {
        let sender = SynapService::open_memory().unwrap();
        sender
            .seal_relay_payload_for(recipient_mailbox_public_key, b"relay-payload")
            .unwrap()
    }

    fn relay_sync_test_envelope(
        sender: &SynapService,
        recipient_mailbox_public_key: [u8; 32],
        payload: RelaySyncEnvelope,
    ) -> Vec<u8> {
        let bytes = postcard::to_allocvec(&payload).unwrap();
        sender
            .seal_relay_payload_for(recipient_mailbox_public_key, &bytes)
            .unwrap()
    }

    fn sender_signing_public_key_from_envelope(bytes: &[u8]) -> [u8; 32] {
        crypto::inspect_verified(bytes)
            .unwrap()
            .sender_signing_public_key
    }

    struct FakeServer {
        addr: String,
        done_rx: mpsc::Receiver<()>,
    }

    impl FakeServer {
        fn base_url(&self) -> String {
            format!("http://{}", self.addr)
        }
    }

    impl Drop for FakeServer {
        fn drop(&mut self) {
            let _ = self.done_rx.recv_timeout(Duration::from_secs(1));
        }
    }

    fn fake_server(handler: impl FnOnce(std::net::TcpStream) + Send + 'static) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (done_tx, done_rx) = mpsc::channel();

        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handler(stream);
            let _ = done_tx.send(());
        });

        FakeServer { addr, done_rx }
    }

    fn capture_post_bodies_server(
        responses: Vec<FakeResponse>,
        captured: std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
    ) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (done_tx, done_rx) = mpsc::channel();

        thread::spawn(move || {
            for response in responses {
                let (stream, _) = listener.accept().unwrap();
                let (_, body) = read_http_request_with_body(&stream);
                captured.lock().unwrap().push(body);
                write_http_response(stream, &response.bytes);
            }
            let _ = done_tx.send(());
        });

        FakeServer { addr, done_rx }
    }

    fn multi_request_server(responses: Vec<FakeResponse>) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (done_tx, done_rx) = mpsc::channel();

        thread::spawn(move || {
            for response in responses {
                let (stream, _) = listener.accept().unwrap();
                read_http_request(&stream);
                write_http_response(stream, &response.bytes);
            }
            let _ = done_tx.send(());
        });

        FakeServer { addr, done_rx }
    }

    struct FakeResponse {
        bytes: Vec<u8>,
    }

    impl FakeResponse {
        fn accepted() -> Self {
            Self {
                bytes: b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n".to_vec(),
            }
        }

        fn ack_no_content() -> Self {
            Self {
                bytes: b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n".to_vec(),
            }
        }

        fn mailbox_empty() -> Self {
            let body = r#"{"code":"mailbox_empty","error":"mailbox is empty"}"#;
            Self {
                bytes: format!(
                    "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                    body.len()
                )
                .into_bytes(),
            }
        }

        fn mailbox_ok(sender_ed25519_hex: String, lease_id: &str, body: Vec<u8>) -> Self {
            let mut response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\n{}: {}\r\n{}: {}\r\n{}: 1711111111\r\ncontent-length: {}\r\n\r\n",
                MESSAGE_SENDER_HEADER,
                sender_ed25519_hex,
                LEASE_ID_HEADER,
                lease_id,
                LEASED_UNTIL_HEADER,
                body.len()
            )
            .into_bytes();
            response.extend_from_slice(&body);
            Self { bytes: response }
        }
    }

    fn write_http_response(mut stream: std::net::TcpStream, bytes: &[u8]) {
        use std::io::Write;
        let _ = stream.write_all(bytes);
        let _ = stream.flush();
    }

    fn write_json_response(stream: std::net::TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        write_http_response(stream, response.as_bytes());
    }

    fn read_http_request(mut stream: &std::net::TcpStream) -> String {
        read_http_request_with_body(&mut stream).0
    }

    fn read_http_request_with_body(mut stream: &std::net::TcpStream) -> (String, Vec<u8>) {
        use std::io::Read;

        let mut header_bytes = Vec::new();
        let mut buf = [0u8; 1];
        while stream.read(&mut buf).ok() == Some(1) {
            header_bytes.push(buf[0]);
            if header_bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        let headers = String::from_utf8_lossy(&header_bytes);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.strip_prefix("content-length: ")
                    .or_else(|| line.strip_prefix("Content-Length: "))
            })
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        let _ = stream.read_exact(&mut body);
        (headers.into_owned(), body)
    }

    fn open_test_payload(
        recipient: &SynapService,
        envelope_bytes: &[u8],
    ) -> Option<RelaySyncEnvelope> {
        let payload: Vec<u8> = recipient
            .with_read(|tx, _reader| {
                let crypto_reader = crate::models::crypto::CryptoReader::new(tx)?;
                Ok(
                    crypto::open_for_local_recipient(&crypto_reader, envelope_bytes)
                        .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?
                        .payload,
                )
            })
            .ok()?;
        postcard::from_bytes(&payload).ok()
    }

    fn open_test_payload_for_any(
        recipient_a: &SynapService,
        recipient_b: &SynapService,
        envelope_bytes: &[u8],
    ) -> RelaySyncEnvelope {
        open_test_payload(recipient_a, envelope_bytes)
            .or_else(|| open_test_payload(recipient_b, envelope_bytes))
            .unwrap()
    }
}
