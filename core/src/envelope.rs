use std::{borrow::Cow, io};

use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305, XNonce};
use rand::random;
use thiserror::Error;

pub const ENVELOPE_MAGIC: [u8; 4] = *b"SKV!";

const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_FLAGS: u16 = 0;
const ENVELOPE_HEADER_LEN: usize = 24;
const XCHACHA20_NONCE_LEN: usize = 24;

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeEncryptionConfig {
    pub key: [u8; 32],
}

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeConfig {
    pub compression_threshold_bytes: usize,
    pub max_decompressed_bytes: usize,
    pub max_envelope_depth: usize,
    pub encryption: Option<EnvelopeEncryptionConfig>,
}

impl EnvelopeConfig {
    pub const DEFAULT: Self = Self {
        compression_threshold_bytes: 256,
        max_decompressed_bytes: 64 * 1024 * 1024,
        max_envelope_depth: 4,
        encryption: None,
    };

    pub const fn with_encryption(mut self, encryption: EnvelopeEncryptionConfig) -> Self {
        self.encryption = Some(encryption);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    config: EnvelopeConfig,
}

impl Envelope {
    pub const fn new(config: EnvelopeConfig) -> Self {
        Self { config }
    }

    pub fn encode_bytes(&self, payload: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        encode_bytes(payload, &self.config)
    }

    pub fn decode_bytes<'a>(&self, bytes: &'a [u8]) -> Result<Cow<'a, [u8]>, EnvelopeError> {
        decode_bytes(bytes, &self.config)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum EnvelopeKind {
    Plain = 0,
    Lz4 = 1,
    XChaCha20Poly1305 = 2,
}

impl TryFrom<u8> for EnvelopeKind {
    type Error = EnvelopeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Lz4),
            2 => Ok(Self::XChaCha20Poly1305),
            other => Err(EnvelopeError::UnknownEnvelopeKind(other)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EnvelopeHeader {
    kind: EnvelopeKind,
    raw_len: u64,
}

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("unsupported envelope version: {0}")]
    UnsupportedEnvelopeVersion(u8),

    #[error("unknown envelope kind: {0}")]
    UnknownEnvelopeKind(u8),

    #[error("invalid envelope: {0}")]
    InvalidEnvelope(&'static str),

    #[error("envelope payload too large: {actual} bytes exceeds {max} bytes")]
    PayloadTooLarge { actual: usize, max: usize },

    #[error("envelope nesting exceeds limit {0}")]
    EnvelopeDepthExceeded(usize),

    #[error("lz4 decompression failed: {0}")]
    Decompress(lz4_flex::block::DecompressError),

    #[error("envelope requires an encryption key but none was provided")]
    MissingEncryptionKey,

    #[error("xchacha20poly1305 operation failed")]
    Aead,
}

impl From<EnvelopeError> for io::Error {
    fn from(value: EnvelopeError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, value)
    }
}

pub fn encode_bytes(payload: &[u8], config: &EnvelopeConfig) -> Result<Vec<u8>, EnvelopeError> {
    let mut encoded = encode_envelope_layer(EnvelopeKind::Plain, payload, payload.len())?;

    if payload.len() >= config.compression_threshold_bytes {
        let compressed = lz4_flex::block::compress(&encoded);
        if compressed.len() + ENVELOPE_HEADER_LEN < encoded.len() {
            encoded = encode_envelope_layer(EnvelopeKind::Lz4, &compressed, encoded.len())?;
        }
    }

    if let Some(encryption) = config.encryption {
        encoded = encode_encrypted_layer(&encoded, encryption)?;
    }

    Ok(encoded)
}

pub fn decode_bytes<'a>(
    bytes: &'a [u8],
    config: &EnvelopeConfig,
) -> Result<Cow<'a, [u8]>, EnvelopeError> {
    if has_envelope_magic(bytes) {
        decode_envelope_bytes(bytes, 0, config)
    } else {
        Ok(Cow::Borrowed(bytes))
    }
}

pub fn has_envelope_magic(bytes: &[u8]) -> bool {
    bytes.len() >= ENVELOPE_MAGIC.len() && bytes[..ENVELOPE_MAGIC.len()] == ENVELOPE_MAGIC
}

fn encode_encrypted_layer(
    inner: &[u8],
    encryption: EnvelopeEncryptionConfig,
) -> Result<Vec<u8>, EnvelopeError> {
    let cipher =
        XChaCha20Poly1305::new_from_slice(&encryption.key).map_err(|_| EnvelopeError::Aead)?;
    let nonce_bytes = random::<[u8; XCHACHA20_NONCE_LEN]>();
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, inner)
        .map_err(|_| EnvelopeError::Aead)?;

    let mut payload = Vec::with_capacity(XCHACHA20_NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    encode_envelope_layer(EnvelopeKind::XChaCha20Poly1305, &payload, inner.len())
}

fn decode_envelope_bytes<'a>(
    bytes: &'a [u8],
    depth: usize,
    config: &EnvelopeConfig,
) -> Result<Cow<'a, [u8]>, EnvelopeError> {
    if depth >= config.max_envelope_depth {
        return Err(EnvelopeError::EnvelopeDepthExceeded(
            config.max_envelope_depth,
        ));
    }

    let (header, payload) = parse_envelope(bytes)?;

    match header.kind {
        EnvelopeKind::Plain => Ok(Cow::Borrowed(payload)),
        EnvelopeKind::Lz4 => {
            let expected_len = usize::try_from(header.raw_len)
                .map_err(|_| EnvelopeError::InvalidEnvelope("raw length does not fit usize"))?;
            if expected_len > config.max_decompressed_bytes {
                return Err(EnvelopeError::PayloadTooLarge {
                    actual: expected_len,
                    max: config.max_decompressed_bytes,
                });
            }

            let decompressed = lz4_flex::block::decompress(payload, expected_len)
                .map_err(EnvelopeError::Decompress)?;
            if decompressed.len() != expected_len {
                return Err(EnvelopeError::InvalidEnvelope(
                    "decompressed payload length mismatch",
                ));
            }

            let nested = decode_envelope_bytes(&decompressed, depth + 1, config)?;
            Ok(Cow::Owned(nested.into_owned()))
        }
        EnvelopeKind::XChaCha20Poly1305 => {
            let encryption = config
                .encryption
                .ok_or(EnvelopeError::MissingEncryptionKey)?;
            let expected_len = usize::try_from(header.raw_len)
                .map_err(|_| EnvelopeError::InvalidEnvelope("raw length does not fit usize"))?;
            if payload.len() < XCHACHA20_NONCE_LEN {
                return Err(EnvelopeError::InvalidEnvelope(
                    "encrypted envelope missing nonce",
                ));
            }

            let cipher = XChaCha20Poly1305::new_from_slice(&encryption.key)
                .map_err(|_| EnvelopeError::Aead)?;
            let nonce = XNonce::from_slice(&payload[..XCHACHA20_NONCE_LEN]);
            let decrypted = cipher
                .decrypt(nonce, &payload[XCHACHA20_NONCE_LEN..])
                .map_err(|_| EnvelopeError::Aead)?;

            if decrypted.len() != expected_len {
                return Err(EnvelopeError::InvalidEnvelope(
                    "decrypted payload length mismatch",
                ));
            }

            let nested = decode_envelope_bytes(&decrypted, depth + 1, config)?;
            Ok(Cow::Owned(nested.into_owned()))
        }
    }
}

fn parse_envelope(bytes: &[u8]) -> Result<(EnvelopeHeader, &[u8]), EnvelopeError> {
    if bytes.len() < ENVELOPE_HEADER_LEN {
        return Err(EnvelopeError::InvalidEnvelope("truncated envelope header"));
    }
    if !has_envelope_magic(bytes) {
        return Err(EnvelopeError::InvalidEnvelope("missing envelope magic"));
    }

    let version = bytes[4];
    if version != ENVELOPE_VERSION {
        return Err(EnvelopeError::UnsupportedEnvelopeVersion(version));
    }

    let kind = EnvelopeKind::try_from(bytes[5])?;
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    if flags != ENVELOPE_FLAGS {
        return Err(EnvelopeError::InvalidEnvelope("unsupported envelope flags"));
    }

    let payload_len = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let raw_len = u64::from_le_bytes(bytes[16..24].try_into().unwrap());

    let actual_payload_len = bytes.len() - ENVELOPE_HEADER_LEN;
    if payload_len
        != u64::try_from(actual_payload_len)
            .map_err(|_| EnvelopeError::InvalidEnvelope("payload length does not fit u64"))?
    {
        return Err(EnvelopeError::InvalidEnvelope("payload length mismatch"));
    }

    match kind {
        EnvelopeKind::Plain if raw_len != payload_len => {
            return Err(EnvelopeError::InvalidEnvelope(
                "plain envelope raw length mismatch",
            ));
        }
        EnvelopeKind::Lz4 | EnvelopeKind::XChaCha20Poly1305 if raw_len == 0 => {
            return Err(EnvelopeError::InvalidEnvelope(
                "wrapped envelope raw length cannot be zero",
            ));
        }
        _ => {}
    }

    Ok((
        EnvelopeHeader { kind, raw_len },
        &bytes[ENVELOPE_HEADER_LEN..],
    ))
}

fn encode_envelope_layer(
    kind: EnvelopeKind,
    payload: &[u8],
    raw_len: usize,
) -> Result<Vec<u8>, EnvelopeError> {
    let payload_len = u64::try_from(payload.len())
        .map_err(|_| EnvelopeError::InvalidEnvelope("payload length does not fit u64"))?;
    let raw_len = u64::try_from(raw_len)
        .map_err(|_| EnvelopeError::InvalidEnvelope("raw length does not fit u64"))?;

    let mut encoded = Vec::with_capacity(ENVELOPE_HEADER_LEN + payload.len());
    encoded.extend_from_slice(&ENVELOPE_MAGIC);
    encoded.push(ENVELOPE_VERSION);
    encoded.push(kind as u8);
    encoded.extend_from_slice(&ENVELOPE_FLAGS.to_le_bytes());
    encoded.extend_from_slice(&payload_len.to_le_bytes());
    encoded.extend_from_slice(&raw_len.to_le_bytes());
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_plain_payload_still_decodes() {
        let payload = b"legacy-payload";
        let decoded = decode_bytes(payload, &EnvelopeConfig::DEFAULT).unwrap();
        assert_eq!(decoded.as_ref(), payload);
    }

    #[test]
    fn envelope_struct_wraps_free_functions() {
        let payload = b"hello envelope";
        let envelope = Envelope::new(EnvelopeConfig::DEFAULT);
        let encoded = envelope.encode_bytes(payload).unwrap();
        let decoded = envelope.decode_bytes(&encoded).unwrap();

        assert!(has_envelope_magic(&encoded));
        assert_eq!(decoded.as_ref(), payload);
    }

    #[test]
    fn compressed_payload_must_decode_to_original_payload() {
        let payload = b"x".repeat(4096);
        let encoded = encode_bytes(&payload, &EnvelopeConfig::DEFAULT).unwrap();
        let decoded = decode_bytes(&encoded, &EnvelopeConfig::DEFAULT).unwrap();

        assert!(encoded.len() < payload.len() + ENVELOPE_HEADER_LEN);
        assert_eq!(decoded.as_ref(), payload.as_slice());
    }

    #[test]
    fn encrypted_payload_must_decode_to_original_payload() {
        let payload = b"synap-secret-payload";
        let config =
            EnvelopeConfig::DEFAULT.with_encryption(EnvelopeEncryptionConfig { key: [9u8; 32] });

        let encoded = encode_bytes(payload, &config).unwrap();
        let decoded = decode_bytes(&encoded, &config).unwrap();

        assert!(has_envelope_magic(&encoded));
        assert_eq!(decoded.as_ref(), payload);
    }

    #[test]
    fn encrypted_and_compressed_payload_must_decode_to_original_payload() {
        let payload = b"secret-".repeat(1024);
        let config =
            EnvelopeConfig::DEFAULT.with_encryption(EnvelopeEncryptionConfig { key: [3u8; 32] });

        let encoded = encode_bytes(&payload, &config).unwrap();
        let decoded = decode_bytes(&encoded, &config).unwrap();

        assert_eq!(decoded.as_ref(), payload.as_slice());
    }

    #[test]
    fn encrypted_envelope_requires_key() {
        let payload = b"secret";
        let encrypted = encode_bytes(
            payload,
            &EnvelopeConfig::DEFAULT.with_encryption(EnvelopeEncryptionConfig { key: [7u8; 32] }),
        )
        .unwrap();

        assert!(matches!(
            decode_bytes(&encrypted, &EnvelopeConfig::DEFAULT),
            Err(EnvelopeError::MissingEncryptionKey)
        ));
    }
}
