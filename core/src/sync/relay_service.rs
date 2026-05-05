use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hex::FromHex;
use serde::Serialize;
use thiserror::Error;

use crate::{
    crypto,
    error::ServiceError,
    service::SynapService,
};

const AUTH_TIMESTAMP_HEADER: &str = "x-synap-timestamp";
const AUTH_SIGNATURE_HEADER: &str = "x-synap-signature";
const MESSAGE_SENDER_HEADER: &str = "x-synap-sender-ed25519";
const LEASE_ID_HEADER: &str = "x-synap-lease-id";
const LEASED_UNTIL_HEADER: &str = "x-synap-leased-until";

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

pub struct RelayHttpService<'a> {
    core: &'a SynapService,
    base_url: String,
    timeout: Duration,
}

#[derive(Debug, Error)]
pub enum RelayHttpError {
    #[error(transparent)]
    Service(#[from] ServiceError),

    #[error("local signing identity is missing")]
    MissingLocalSigningIdentity,

    #[error("http transport error: {0}")]
    Transport(String),

    #[error("relay returned unexpected status {status}: {body}")]
    HttpStatus { status: u16, body: String },

    #[error("relay mailbox is empty")]
    EmptyMailbox,

    #[error("relay response missing header: {0}")]
    MissingHeader(&'static str),

    #[error("relay response header {0} is invalid")]
    InvalidHeader(&'static str),

    #[error("relay returned malformed sender public key")]
    InvalidSenderPublicKey,

    #[error("relay ack was rejected")]
    AckRejected,

    #[error("sealed envelope error: {0}")]
    SealedEnvelope(String),
}

impl<'a> RelayHttpService<'a> {
    pub fn new(core: &'a SynapService, base_url: impl Into<String>) -> Self {
        Self {
            core,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            timeout: Duration::from_secs(10),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn post_envelope(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
        envelope_bytes: &[u8],
    ) -> Result<(), RelayHttpError> {
        let url = self.mailbox_url(recipient_mailbox_ed25519);
        let mut response = ureq::post(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header("content-type", "application/octet-stream")
            .send(envelope_bytes)
            .map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        if status == 202 {
            return Ok(());
        }

        let body = response
            .body_mut()
            .read_to_string()
            .map_err(map_ureq_error)?;
        Err(RelayHttpError::HttpStatus { status, body })
    }

    pub fn get_envelope(
        &self,
        recipient_mailbox_ed25519: [u8; 32],
    ) -> Result<Option<RelayLeasedEnvelope>, RelayHttpError> {
        let timestamp = now_ms().to_string();
        let auth = self.mailbox_auth(recipient_mailbox_ed25519, "GET", &timestamp)?;
        let url = self.mailbox_url(recipient_mailbox_ed25519);

        let mut response = ureq::get(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header(AUTH_TIMESTAMP_HEADER, &timestamp)
            .header(AUTH_SIGNATURE_HEADER, &auth.signature_hex)
            .call()
            .map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(map_ureq_error)?;
            return Err(RelayHttpError::HttpStatus { status, body });
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

        let envelope_bytes = response
            .body_mut()
            .read_to_vec()
            .map_err(map_ureq_error)?;

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
                crypto::open_for_local_recipient(&crypto_reader, &leased.envelope_bytes)
                    .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
            })
            .map_err(RelayHttpError::Service)?;

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

        let mut response = ureq::post(&url)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header(AUTH_TIMESTAMP_HEADER, &timestamp)
            .header(AUTH_SIGNATURE_HEADER, &auth.signature_hex)
            .header("content-type", "application/json")
            .send(body.as_str())
            .map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        match status {
            204 => Ok(()),
            404 => Err(RelayHttpError::AckRejected),
            _ => {
                let body = response
                    .body_mut()
                    .read_to_string()
                    .map_err(map_ureq_error)?;
                Err(RelayHttpError::HttpStatus { status, body })
            }
        }
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
        let signature = self.core.sign_relay_mailbox_auth(payload.as_bytes()).map_err(|error| {
            match error {
                ServiceError::Other(other)
                    if other.to_string().contains("local signing identity is missing") =>
                {
                    RelayHttpError::MissingLocalSigningIdentity
                }
                other => RelayHttpError::Service(other),
            }
        })?;

        Ok(RelayMailboxAuth {
            signature_hex: hex::encode(signature),
        })
    }
}

struct RelayMailboxAuth {
    signature_hex: String,
}

#[derive(Debug, Serialize)]
struct RelayAckRequest {
    sender_ed25519: String,
    lease_id: String,
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

fn map_ureq_error(err: ureq::Error) -> RelayHttpError {
    match err {
        ureq::Error::StatusCode(status) => RelayHttpError::HttpStatus {
            status,
            body: String::new(),
        },
        other => RelayHttpError::Transport(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, sync::mpsc, thread};

    use crate::{
        crypto::seal_for_recipient,
        service::SynapService,
    };

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
        let recipient_x25519 = service.local_relay_recipient_identity_public_key().unwrap();
        let envelope = sender_test_envelope(recipient_x25519);
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
    fn ack_envelope_returns_rejected_for_404() {
        let service = SynapService::open_memory().unwrap();
        let recipient_mailbox = service.local_relay_mailbox_public_key().unwrap();

        let server = fake_server(|stream| {
            read_http_request(&stream);
            write_http_response(
                stream,
                b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n",
            );
        });

        let client = RelayHttpService::new(&service, server.base_url());
        let result = client.ack_envelope(recipient_mailbox, [0x22; 32], "lease-1");

        assert!(matches!(result, Err(RelayHttpError::AckRejected)));
    }

    fn sender_test_envelope(recipient_identity_public_key: [u8; 32]) -> Vec<u8> {
        let sender = SynapService::open_memory().unwrap();
        sender
            .with_read(|tx, _reader| {
                let crypto_reader = crate::models::crypto::CryptoReader::new(tx)?;
                seal_for_recipient(&crypto_reader, recipient_identity_public_key, b"relay-payload")
                    .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
            })
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

    fn write_http_response(mut stream: std::net::TcpStream, bytes: &[u8]) {
        use std::io::Write;
        let _ = stream.write_all(bytes);
        let _ = stream.flush();
    }

    fn read_http_request(mut stream: &std::net::TcpStream) {
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
    }
}
