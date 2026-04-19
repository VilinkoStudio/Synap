use std::io::{self, Read, Write};

use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::{
    crypto::{
        get_trusted_public_key_by_bytes, local_signing_public_key, public_key_fingerprint,
        sign_with_local_identity, verify_signed_bytes, TrustedPublicKeyRecord,
    },
    models::crypto::CryptoReader,
};

const CHANNEL_PROTOCOL_VERSION: u8 = 1;
const HANDSHAKE_CONTEXT: &[u8] = b"synap.crypto.channel.handshake.v1";
const KEY_DERIVATION_CONTEXT: &[u8] = b"synap.crypto.channel.key-derivation.v1";
const MAX_FRAME_LEN: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoChannelMode {
    Plaintext,
    Encrypted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelRole {
    Initiator,
    Acceptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPeer {
    pub signing_public_key: [u8; 32],
    pub trust_record: TrustedPublicKeyRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    pub key_id: Uuid,
    pub public_key: [u8; 32],
    pub fingerprint: [u8; 32],
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CryptoChannelOptions {
    pub mode: CryptoChannelMode,
    pub max_frame_len: usize,
    pub expected_signing_public_key: Option<[u8; 32]>,
}

impl Default for CryptoChannelOptions {
    fn default() -> Self {
        Self {
            mode: CryptoChannelMode::Plaintext,
            max_frame_len: MAX_FRAME_LEN,
            expected_signing_public_key: None,
        }
    }
}

#[derive(Debug)]
struct ChannelCipher {
    send_key: [u8; 32],
    recv_key: [u8; 32],
    send_counter: u64,
}

pub struct CryptoChannel<T> {
    inner: T,
    peer: AuthenticatedPeer,
    mode: CryptoChannelMode,
    cipher: Option<ChannelCipher>,
    max_frame_len: usize,
    read_buffer: Vec<u8>,
    read_offset: usize,
    write_buffer: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CryptoChannelError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("database error: {0}")]
    Db(#[from] redb::Error),

    #[error("local signing identity is missing")]
    MissingLocalSigningIdentity,

    #[error("peer handshake protocol version mismatch: {0}")]
    UnsupportedVersion(u8),

    #[error("peer handshake mode mismatch")]
    ModeMismatch,

    #[error("peer trust verification failed")]
    UntrustedPeer {
        public_key: [u8; 32],
        fingerprint: [u8; 32],
    },

    #[error("peer signing public key does not match expected identity")]
    PeerIdentityMismatch {
        expected_public_key: [u8; 32],
        actual_public_key: [u8; 32],
        actual_fingerprint: [u8; 32],
    },

    #[error("peer signature verification failed")]
    InvalidPeerSignature,

    #[error("peer handshake is malformed: {0}")]
    InvalidHandshake(&'static str),

    #[error("channel frame is too large: {actual} > {max}")]
    FrameTooLarge { actual: usize, max: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandshakeHello {
    version: u8,
    encrypt: bool,
    signing_public_key: [u8; 32],
    key_exchange_public_key: Option<[u8; 32]>,
    signature: Vec<u8>,
}

impl<T> CryptoChannel<T>
where
    T: Read + Write + Send,
{
    pub fn connect(
        inner: T,
        reader: &CryptoReader<'_>,
        options: CryptoChannelOptions,
    ) -> Result<Self, CryptoChannelError> {
        Self::handshake(inner, reader, ChannelRole::Initiator, options)
    }

    pub fn accept(
        inner: T,
        reader: &CryptoReader<'_>,
        options: CryptoChannelOptions,
    ) -> Result<Self, CryptoChannelError> {
        Self::handshake(inner, reader, ChannelRole::Acceptor, options)
    }

    pub fn peer(&self) -> &AuthenticatedPeer {
        &self.peer
    }

    pub fn mode(&self) -> CryptoChannelMode {
        self.mode
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), CryptoChannelError> {
        self.send_frame(bytes)
    }

    fn handshake(
        mut inner: T,
        reader: &CryptoReader<'_>,
        role: ChannelRole,
        options: CryptoChannelOptions,
    ) -> Result<Self, CryptoChannelError> {
        let signing_public_key = local_signing_public_key(reader)?
            .ok_or(CryptoChannelError::MissingLocalSigningIdentity)?;

        let local_x25519_secret = match options.mode {
            CryptoChannelMode::Plaintext => None,
            CryptoChannelMode::Encrypted => Some(StaticSecret::random()),
        };
        let local_x25519_public = local_x25519_secret
            .as_ref()
            .map(X25519PublicKey::from)
            .map(|key| key.to_bytes());

        let local_hello = sign_handshake_hello(
            reader,
            options.mode,
            signing_public_key,
            local_x25519_public,
        )?;

        write_packet(
            &mut inner,
            &postcard::to_allocvec(&local_hello).map_err(invalid_data)?,
        )?;
        inner.flush()?;

        let peer_bytes = read_packet(&mut inner, options.max_frame_len)?;
        let peer_hello: HandshakeHello = postcard::from_bytes(&peer_bytes).map_err(invalid_data)?;
        validate_peer_hello(&peer_hello, options.mode)?;

        if let Some(expected_public_key) = options.expected_signing_public_key {
            if peer_hello.signing_public_key != expected_public_key {
                return Err(CryptoChannelError::PeerIdentityMismatch {
                    expected_public_key,
                    actual_public_key: peer_hello.signing_public_key,
                    actual_fingerprint: public_key_fingerprint(&peer_hello.signing_public_key),
                });
            }
        }

        let trust_record = get_trusted_public_key_by_bytes(reader, peer_hello.signing_public_key)?
            .ok_or(CryptoChannelError::UntrustedPeer {
                public_key: peer_hello.signing_public_key,
                fingerprint: public_key_fingerprint(&peer_hello.signing_public_key),
            })?;

        let cipher = match options.mode {
            CryptoChannelMode::Plaintext => None,
            CryptoChannelMode::Encrypted => {
                let local_secret = local_x25519_secret.ok_or(
                    CryptoChannelError::InvalidHandshake("missing local x25519 secret"),
                )?;
                let peer_public_bytes = peer_hello.key_exchange_public_key.ok_or(
                    CryptoChannelError::InvalidHandshake("missing peer x25519 public key"),
                )?;
                let peer_public = X25519PublicKey::from(peer_public_bytes);
                let shared_secret = local_secret.diffie_hellman(&peer_public);
                Some(derive_cipher(
                    role,
                    shared_secret.as_bytes(),
                    local_hello.key_exchange_public_key.ok_or(
                        CryptoChannelError::InvalidHandshake("missing local x25519 public key"),
                    )?,
                    peer_public_bytes,
                )?)
            }
        };

        Ok(Self {
            inner,
            peer: AuthenticatedPeer {
                signing_public_key: peer_hello.signing_public_key,
                trust_record,
            },
            mode: options.mode,
            cipher,
            max_frame_len: options.max_frame_len,
            read_buffer: Vec::new(),
            read_offset: 0,
            write_buffer: Vec::new(),
        })
    }

    fn send_frame(&mut self, plain: &[u8]) -> Result<(), CryptoChannelError> {
        if plain.len() > self.max_frame_len {
            return Err(CryptoChannelError::FrameTooLarge {
                actual: plain.len(),
                max: self.max_frame_len,
            });
        }

        let payload =
            if let Some(cipher) = &mut self.cipher {
                let counter = cipher.send_counter;
                cipher.send_counter = cipher.send_counter.checked_add(1).ok_or(
                    CryptoChannelError::InvalidHandshake("send counter overflow"),
                )?;

                let ciphertext =
                    seal_bytes_xchacha20(&cipher.send_key, counter, &counter.to_le_bytes(), plain)
                        .map_err(invalid_data)?;
                let mut payload = Vec::with_capacity(8 + ciphertext.len());
                payload.extend_from_slice(&counter.to_le_bytes());
                payload.extend_from_slice(&ciphertext);
                payload
            } else {
                plain.to_vec()
            };

        write_packet(&mut self.inner, &payload)?;
        self.inner.flush()?;
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Vec<u8>, CryptoChannelError> {
        let payload = read_packet(&mut self.inner, self.max_frame_len)?;
        if let Some(cipher) = &self.cipher {
            if payload.len() < 8 {
                return Err(CryptoChannelError::InvalidHandshake(
                    "encrypted frame missing nonce counter",
                ));
            }
            let counter = u64::from_le_bytes(payload[..8].try_into().unwrap());
            let ciphertext = &payload[8..];
            let plain = open_bytes_xchacha20(
                &cipher.recv_key,
                counter,
                &counter.to_le_bytes(),
                ciphertext,
            )
            .map_err(invalid_data)?;
            if plain.len() > self.max_frame_len {
                return Err(CryptoChannelError::FrameTooLarge {
                    actual: plain.len(),
                    max: self.max_frame_len,
                });
            }
            Ok(plain)
        } else {
            Ok(payload)
        }
    }
}

impl AuthenticatedPeer {
    pub fn identity(&self) -> PeerIdentity {
        PeerIdentity {
            key_id: self.trust_record.id,
            public_key: self.trust_record.public_key,
            fingerprint: self.trust_record.fingerprint,
            label: self.trust_record.note.clone(),
        }
    }

    pub fn identity_label(&self) -> Option<&str> {
        self.trust_record.note.as_deref()
    }
}

impl<T> Read for CryptoChannel<T>
where
    T: Read + Write + Send,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.read_offset >= self.read_buffer.len() {
            match self.recv_frame() {
                Ok(frame) => {
                    self.read_buffer = frame;
                    self.read_offset = 0;
                }
                Err(CryptoChannelError::Io(err)) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    return Ok(0);
                }
                Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
            }
        }

        let available = &self.read_buffer[self.read_offset..];
        let to_copy = available.len().min(buf.len());
        buf[..to_copy].copy_from_slice(&available[..to_copy]);
        self.read_offset += to_copy;

        if self.read_offset >= self.read_buffer.len() {
            self.read_buffer.clear();
            self.read_offset = 0;
        }

        Ok(to_copy)
    }
}

impl<T> Write for CryptoChannel<T>
where
    T: Read + Write + Send,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.write_buffer.is_empty() {
            let frame = std::mem::take(&mut self.write_buffer);
            self.send_frame(&frame)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        }
        self.inner.flush()
    }
}

fn sign_handshake_hello(
    reader: &CryptoReader<'_>,
    mode: CryptoChannelMode,
    signing_public_key: [u8; 32],
    key_exchange_public_key: Option<[u8; 32]>,
) -> Result<HandshakeHello, CryptoChannelError> {
    let payload = handshake_signing_payload(mode, signing_public_key, key_exchange_public_key);
    let signature = sign_with_local_identity(reader, &payload)?
        .ok_or(CryptoChannelError::MissingLocalSigningIdentity)?;
    Ok(HandshakeHello {
        version: CHANNEL_PROTOCOL_VERSION,
        encrypt: mode == CryptoChannelMode::Encrypted,
        signing_public_key,
        key_exchange_public_key,
        signature: signature.to_vec(),
    })
}

fn validate_peer_hello(
    hello: &HandshakeHello,
    local_mode: CryptoChannelMode,
) -> Result<(), CryptoChannelError> {
    if hello.version != CHANNEL_PROTOCOL_VERSION {
        return Err(CryptoChannelError::UnsupportedVersion(hello.version));
    }

    let peer_mode = if hello.encrypt {
        CryptoChannelMode::Encrypted
    } else {
        CryptoChannelMode::Plaintext
    };

    if peer_mode != local_mode {
        return Err(CryptoChannelError::ModeMismatch);
    }

    match (peer_mode, hello.key_exchange_public_key) {
        (CryptoChannelMode::Plaintext, Some(_)) => {
            return Err(CryptoChannelError::InvalidHandshake(
                "plaintext mode must not carry x25519 key",
            ));
        }
        (CryptoChannelMode::Encrypted, None) => {
            return Err(CryptoChannelError::InvalidHandshake(
                "encrypted mode requires x25519 key",
            ));
        }
        _ => {}
    }

    let payload = handshake_signing_payload(
        peer_mode,
        hello.signing_public_key,
        hello.key_exchange_public_key,
    );
    let signature: [u8; 64] = hello.signature.as_slice().try_into().map_err(|_| {
        CryptoChannelError::InvalidHandshake("peer ed25519 signature must be 64 bytes")
    })?;
    if !verify_signed_bytes(hello.signing_public_key, &payload, signature) {
        return Err(CryptoChannelError::InvalidPeerSignature);
    }

    Ok(())
}

fn handshake_signing_payload(
    mode: CryptoChannelMode,
    signing_public_key: [u8; 32],
    key_exchange_public_key: Option<[u8; 32]>,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(HANDSHAKE_CONTEXT.len() + 66);
    payload.extend_from_slice(HANDSHAKE_CONTEXT);
    payload.push(CHANNEL_PROTOCOL_VERSION);
    payload.push(match mode {
        CryptoChannelMode::Plaintext => 0,
        CryptoChannelMode::Encrypted => 1,
    });
    payload.extend_from_slice(&signing_public_key);
    if let Some(key_exchange_public_key) = key_exchange_public_key {
        payload.extend_from_slice(&key_exchange_public_key);
    }
    payload
}

fn derive_cipher(
    role: ChannelRole,
    shared_secret: &[u8; 32],
    local_public_key: [u8; 32],
    peer_public_key: [u8; 32],
) -> Result<ChannelCipher, CryptoChannelError> {
    let mut okm = [0u8; 64];
    let mut info = Vec::with_capacity(KEY_DERIVATION_CONTEXT.len() + 64);
    info.extend_from_slice(KEY_DERIVATION_CONTEXT);

    let (initiator_public, acceptor_public) = match role {
        ChannelRole::Initiator => (local_public_key, peer_public_key),
        ChannelRole::Acceptor => (peer_public_key, local_public_key),
    };
    info.extend_from_slice(&initiator_public);
    info.extend_from_slice(&acceptor_public);

    Hkdf::<Sha256>::new(None, shared_secret)
        .expand(&info, &mut okm)
        .map_err(|_| CryptoChannelError::InvalidHandshake("hkdf output size is invalid"))?;

    let mut first = [0u8; 32];
    let mut second = [0u8; 32];
    first.copy_from_slice(&okm[..32]);
    second.copy_from_slice(&okm[32..]);

    let (send_key, recv_key) = match role {
        ChannelRole::Initiator => (first, second),
        ChannelRole::Acceptor => (second, first),
    };

    Ok(ChannelCipher {
        send_key,
        recv_key,
        send_counter: 0,
    })
}

fn write_packet<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), CryptoChannelError> {
    if bytes.len() > u32::MAX as usize {
        return Err(CryptoChannelError::FrameTooLarge {
            actual: bytes.len(),
            max: u32::MAX as usize,
        });
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    Ok(())
}

fn read_packet<R: Read>(
    reader: &mut R,
    max_frame_len: usize,
) -> Result<Vec<u8>, CryptoChannelError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > max_frame_len {
        return Err(CryptoChannelError::FrameTooLarge {
            actual: len,
            max: max_frame_len,
        });
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> CryptoChannelError {
    CryptoChannelError::Io(io::Error::new(io::ErrorKind::InvalidData, error))
}

fn seal_bytes_xchacha20(
    key: &[u8; 32],
    counter: u64,
    aad: &[u8],
    plain_payload: &[u8],
) -> Result<Vec<u8>, io::Error> {
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid xchacha20 key"))?;
    let nonce = nonce_from_counter(counter);
    cipher
        .encrypt(
            &nonce,
            chacha20poly1305::aead::Payload {
                msg: plain_payload,
                aad,
            },
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "xchacha20 encrypt failed"))
}

fn open_bytes_xchacha20(
    key: &[u8; 32],
    counter: u64,
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, io::Error> {
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid xchacha20 key"))?;
    let nonce = nonce_from_counter(counter);
    cipher
        .decrypt(
            &nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "xchacha20 decrypt failed"))
}

fn nonce_from_counter(counter: u64) -> XNonce {
    let mut nonce = [0u8; 24];
    nonce[16..].copy_from_slice(&counter.to_le_bytes());
    *XNonce::from_slice(&nonce)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;
    use crate::{
        crypto::{
            ensure_local_signing_identity, import_trusted_public_key, local_signing_public_key,
        },
        models::crypto::CryptoWriter,
    };

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    fn make_peer_db(trust_peer: Option<[u8; 32]>) -> (Database, [u8; 32]) {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        ensure_local_signing_identity(&writer).unwrap();
        if let Some(public_key) = trust_peer {
            import_trusted_public_key(&writer, public_key, Some("peer".into())).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let signing_public_key = local_signing_public_key(&reader).unwrap().unwrap();
        drop(rtx);

        (db, signing_public_key)
    }

    #[test]
    fn test_crypto_channel_plaintext_roundtrip() {
        let (server_db, server_signing_public_key) = make_peer_db(None);
        let (client_db, client_signing_public_key) = make_peer_db(Some(server_signing_public_key));

        {
            let wtx = server_db.begin_write().unwrap();
            let writer = CryptoWriter::new(&wtx);
            import_trusted_public_key(&writer, client_signing_public_key, Some("client".into()))
                .unwrap();
            wtx.commit().unwrap();
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let rtx = server_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            let mut channel = CryptoChannel::accept(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut buf = [0u8; 5];
            channel.read_exact(&mut buf).unwrap();
            tx.send(buf).unwrap();
            channel.write_all(b"world").unwrap();
            channel.flush().unwrap();
        });

        let client = thread::spawn(move || {
            let stream = TcpStream::connect(addr).unwrap();
            let rtx = client_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            let mut channel = CryptoChannel::connect(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    ..Default::default()
                },
            )
            .unwrap();
            channel.write_all(b"hello").unwrap();
            channel.flush().unwrap();
            let mut buf = [0u8; 5];
            channel.read_exact(&mut buf).unwrap();
            buf
        });

        assert_eq!(rx.recv().unwrap(), *b"hello");
        assert_eq!(client.join().unwrap(), *b"world");
        server.join().unwrap();
    }

    #[test]
    fn test_crypto_channel_encrypted_roundtrip() {
        let (server_db, server_signing_public_key) = make_peer_db(None);
        let (client_db, client_signing_public_key) = make_peer_db(Some(server_signing_public_key));

        {
            let wtx = server_db.begin_write().unwrap();
            let writer = CryptoWriter::new(&wtx);
            import_trusted_public_key(&writer, client_signing_public_key, Some("client".into()))
                .unwrap();
            wtx.commit().unwrap();
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let rtx = server_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            let mut channel = CryptoChannel::accept(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Encrypted,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut buf = vec![0u8; 12];
            channel.read_exact(&mut buf).unwrap();
            assert_eq!(buf, b"secret hello");
            channel.send(b"secret world").unwrap();
        });

        let client = thread::spawn(move || {
            let stream = TcpStream::connect(addr).unwrap();
            let rtx = client_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            let mut channel = CryptoChannel::connect(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Encrypted,
                    ..Default::default()
                },
            )
            .unwrap();
            channel.send(b"secret hello").unwrap();
            let mut buf = vec![0u8; 12];
            channel.read_exact(&mut buf).unwrap();
            buf
        });

        assert_eq!(client.join().unwrap(), b"secret world");
        server.join().unwrap();
    }

    #[test]
    fn test_crypto_channel_rejects_untrusted_peer() {
        let (server_db, _) = make_peer_db(None);
        let (client_db, _) = make_peer_db(None);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let rtx = server_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            CryptoChannel::accept(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    ..Default::default()
                },
            )
        });

        let client = thread::spawn(move || {
            let stream = TcpStream::connect(addr).unwrap();
            let rtx = client_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            CryptoChannel::connect(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    ..Default::default()
                },
            )
        });

        assert!(matches!(
            server.join().unwrap(),
            Err(CryptoChannelError::UntrustedPeer { .. })
        ));
        assert!(matches!(
            client.join().unwrap(),
            Err(CryptoChannelError::UntrustedPeer { .. }) | Err(CryptoChannelError::Io(_))
        ));
    }

    #[test]
    fn test_crypto_channel_reports_peer_identity_mismatch_after_handshake() {
        let (server_db, server_signing_public_key) = make_peer_db(None);
        let (client_db, client_signing_public_key) = make_peer_db(Some(server_signing_public_key));
        let unexpected_public_key = [77u8; 32];

        {
            let wtx = server_db.begin_write().unwrap();
            let writer = CryptoWriter::new(&wtx);
            import_trusted_public_key(&writer, client_signing_public_key, Some("client".into()))
                .unwrap();
            wtx.commit().unwrap();
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let rtx = server_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            CryptoChannel::accept(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    expected_signing_public_key: Some(unexpected_public_key),
                    ..Default::default()
                },
            )
        });

        let client = thread::spawn(move || {
            let stream = TcpStream::connect(addr).unwrap();
            let rtx = client_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            CryptoChannel::connect(
                stream,
                &reader,
                CryptoChannelOptions {
                    mode: CryptoChannelMode::Plaintext,
                    ..Default::default()
                },
            )
        });

        assert!(matches!(
            server.join().unwrap(),
            Err(CryptoChannelError::PeerIdentityMismatch {
                expected_public_key,
                actual_public_key,
                ..
            }) if expected_public_key == unexpected_public_key
                && actual_public_key == client_signing_public_key
        ));
        assert!(matches!(
            client.join().unwrap(),
            Err(CryptoChannelError::Io(_)) | Ok(_)
        ));
    }
}
