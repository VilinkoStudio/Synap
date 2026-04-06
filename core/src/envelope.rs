use std::{borrow::Cow, io};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub const ENVELOPE_MAGIC: [u8; 4] = *b"SKV!";

const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_FLAGS: u16 = 0;
const ENVELOPE_HEADER_LEN: usize = 24;

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeConfig {
    pub compression_threshold_bytes: usize,
    pub max_decompressed_bytes: usize,
    pub max_envelope_depth: usize,
}

impl EnvelopeConfig {
    pub const DEFAULT: Self = Self {
        compression_threshold_bytes: 256,
        max_decompressed_bytes: 64 * 1024 * 1024,
        max_envelope_depth: 4,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum EnvelopeKind {
    Plain = 0,
    Lz4 = 1,
}

impl TryFrom<u8> for EnvelopeKind {
    type Error = EnvelopeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Lz4),
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
    #[error("failed to serialize value: {0}")]
    Serialize(postcard::Error),

    #[error("failed to deserialize value: {0}")]
    Deserialize(postcard::Error),

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
}

impl From<EnvelopeError> for io::Error {
    fn from(value: EnvelopeError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, value)
    }
}

pub fn encode_postcard<V: Serialize>(value: &V) -> Result<Vec<u8>, EnvelopeError> {
    let plain = postcard::to_allocvec(value).map_err(EnvelopeError::Serialize)?;
    encode_bytes(&plain)
}

pub fn decode_postcard<V: DeserializeOwned>(bytes: &[u8]) -> Result<V, EnvelopeError> {
    let payload = decode_bytes(bytes)?;
    postcard::from_bytes(payload.as_ref()).map_err(EnvelopeError::Deserialize)
}

pub fn encode_bytes(plain_payload: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
    let config = EnvelopeConfig::DEFAULT;
    let plain_layer =
        encode_envelope_layer(EnvelopeKind::Plain, plain_payload, plain_payload.len())?;

    if plain_payload.len() < config.compression_threshold_bytes {
        return Ok(plain_layer);
    }

    let compressed = lz4_flex::block::compress(&plain_layer);

    if compressed.len() + ENVELOPE_HEADER_LEN >= plain_layer.len() {
        return Ok(plain_layer);
    }

    encode_envelope_layer(EnvelopeKind::Lz4, &compressed, plain_layer.len())
}

pub fn decode_bytes(bytes: &[u8]) -> Result<Cow<'_, [u8]>, EnvelopeError> {
    if has_envelope_magic(bytes) {
        decode_envelope_bytes(bytes, 0, &EnvelopeConfig::DEFAULT)
    } else {
        Ok(Cow::Borrowed(bytes))
    }
}

pub fn has_envelope_magic(bytes: &[u8]) -> bool {
    bytes.len() >= ENVELOPE_MAGIC.len() && bytes[..ENVELOPE_MAGIC.len()] == ENVELOPE_MAGIC
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
        EnvelopeKind::Lz4 if raw_len == 0 => {
            return Err(EnvelopeError::InvalidEnvelope(
                "compressed envelope raw length cannot be zero",
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

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Sample {
        title: String,
    }

    #[test]
    fn legacy_plain_postcard_still_decodes() {
        let bytes = postcard::to_allocvec(&Sample {
            title: "legacy".into(),
        })
        .unwrap();

        let decoded: Sample = decode_postcard(&bytes).unwrap();
        assert_eq!(
            decoded,
            Sample {
                title: "legacy".into()
            }
        );
    }

    #[test]
    fn new_payloads_always_have_envelope_magic() {
        let bytes = encode_postcard(&Sample {
            title: "new".into(),
        })
        .unwrap();

        assert!(has_envelope_magic(&bytes));
    }

    #[test]
    fn compressed_payload_must_decode_to_nested_plain_envelope() {
        let sample = Sample {
            title: "x".repeat(4096),
        };
        let plain = postcard::to_allocvec(&sample).unwrap();
        let bytes = encode_postcard(&sample).unwrap();

        assert!(bytes.len() < plain.len());

        let decoded: Sample = decode_postcard(&bytes).unwrap();
        assert_eq!(decoded, sample);
    }
}
