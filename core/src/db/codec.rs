use std::{borrow::Cow, io, io::Cursor};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub(crate) const ENVELOPE_MAGIC: [u8; 4] = *b"SKV!";

const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_FLAGS: u16 = 0;
const ENVELOPE_HEADER_LEN: usize = 24;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ValueCodecConfig {
    pub(crate) compression_threshold_bytes: usize,
    pub(crate) zstd_level: i32,
    pub(crate) max_decompressed_bytes: usize,
    pub(crate) max_envelope_depth: usize,
}

impl ValueCodecConfig {
    pub(crate) const DEFAULT: Self = Self {
        compression_threshold_bytes: 256,
        zstd_level: 3,
        max_decompressed_bytes: 64 * 1024 * 1024,
        max_envelope_depth: 4,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum EnvelopeKind {
    Plain = 0,
    Zstd = 1,
}

impl TryFrom<u8> for EnvelopeKind {
    type Error = ValueCodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Zstd),
            other => Err(ValueCodecError::UnknownEnvelopeKind(other)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EnvelopeHeader {
    kind: EnvelopeKind,
    raw_len: u64,
}

#[derive(Debug, Error)]
pub(crate) enum ValueCodecError {
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

    #[error("zstd compression failed: {0}")]
    Compress(io::Error),

    #[error("zstd decompression failed: {0}")]
    Decompress(io::Error),
}

impl From<ValueCodecError> for io::Error {
    fn from(value: ValueCodecError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, value)
    }
}

pub(crate) fn encode_default<V: Serialize>(value: &V) -> Result<Vec<u8>, ValueCodecError> {
    let plain = postcard::to_allocvec(value).map_err(ValueCodecError::Serialize)?;
    encode_default_bytes(&plain)
}

pub(crate) fn decode_default<V: DeserializeOwned>(bytes: &[u8]) -> Result<V, ValueCodecError> {
    let payload = decode_default_bytes(bytes)?;
    postcard::from_bytes(payload.as_ref()).map_err(ValueCodecError::Deserialize)
}

pub(crate) fn decode_default_bytes(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ValueCodecError> {
    if has_envelope_magic(bytes) {
        decode_envelope_bytes(bytes, 0, &ValueCodecConfig::DEFAULT)
    } else {
        Ok(Cow::Borrowed(bytes))
    }
}

pub(crate) fn has_envelope_magic(bytes: &[u8]) -> bool {
    bytes.len() >= ENVELOPE_MAGIC.len() && bytes[..ENVELOPE_MAGIC.len()] == ENVELOPE_MAGIC
}

fn encode_default_bytes(plain_payload: &[u8]) -> Result<Vec<u8>, ValueCodecError> {
    let config = ValueCodecConfig::DEFAULT;
    let plain_layer =
        encode_envelope_layer(EnvelopeKind::Plain, plain_payload, plain_payload.len())?;

    if plain_payload.len() < config.compression_threshold_bytes {
        return Ok(plain_layer);
    }

    let compressed = zstd::stream::encode_all(Cursor::new(&plain_layer), config.zstd_level)
        .map_err(ValueCodecError::Compress)?;

    if compressed.len() + ENVELOPE_HEADER_LEN >= plain_layer.len() {
        return Ok(plain_layer);
    }

    encode_envelope_layer(EnvelopeKind::Zstd, &compressed, plain_layer.len())
}

fn decode_envelope_bytes<'a>(
    bytes: &'a [u8],
    depth: usize,
    config: &ValueCodecConfig,
) -> Result<Cow<'a, [u8]>, ValueCodecError> {
    if depth >= config.max_envelope_depth {
        return Err(ValueCodecError::EnvelopeDepthExceeded(
            config.max_envelope_depth,
        ));
    }

    let (header, payload) = parse_envelope(bytes)?;

    match header.kind {
        EnvelopeKind::Plain => Ok(Cow::Borrowed(payload)),
        EnvelopeKind::Zstd => {
            let expected_len = usize::try_from(header.raw_len)
                .map_err(|_| ValueCodecError::InvalidEnvelope("raw length does not fit usize"))?;
            if expected_len > config.max_decompressed_bytes {
                return Err(ValueCodecError::PayloadTooLarge {
                    actual: expected_len,
                    max: config.max_decompressed_bytes,
                });
            }

            let decompressed = zstd::stream::decode_all(Cursor::new(payload))
                .map_err(ValueCodecError::Decompress)?;
            if decompressed.len() != expected_len {
                return Err(ValueCodecError::InvalidEnvelope(
                    "decompressed payload length mismatch",
                ));
            }

            let nested = decode_envelope_bytes(&decompressed, depth + 1, config)?;
            Ok(Cow::Owned(nested.into_owned()))
        }
    }
}

fn parse_envelope(bytes: &[u8]) -> Result<(EnvelopeHeader, &[u8]), ValueCodecError> {
    if bytes.len() < ENVELOPE_HEADER_LEN {
        return Err(ValueCodecError::InvalidEnvelope(
            "truncated envelope header",
        ));
    }
    if !has_envelope_magic(bytes) {
        return Err(ValueCodecError::InvalidEnvelope("missing envelope magic"));
    }

    let version = bytes[4];
    if version != ENVELOPE_VERSION {
        return Err(ValueCodecError::UnsupportedEnvelopeVersion(version));
    }

    let kind = EnvelopeKind::try_from(bytes[5])?;
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    if flags != ENVELOPE_FLAGS {
        return Err(ValueCodecError::InvalidEnvelope(
            "unsupported envelope flags",
        ));
    }

    let payload_len = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let raw_len = u64::from_le_bytes(bytes[16..24].try_into().unwrap());

    let actual_payload_len = bytes.len() - ENVELOPE_HEADER_LEN;
    if payload_len
        != u64::try_from(actual_payload_len)
            .map_err(|_| ValueCodecError::InvalidEnvelope("payload length does not fit u64"))?
    {
        return Err(ValueCodecError::InvalidEnvelope("payload length mismatch"));
    }

    match kind {
        EnvelopeKind::Plain if raw_len != payload_len => {
            return Err(ValueCodecError::InvalidEnvelope(
                "plain envelope raw length mismatch",
            ));
        }
        EnvelopeKind::Zstd if raw_len == 0 => {
            return Err(ValueCodecError::InvalidEnvelope(
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
) -> Result<Vec<u8>, ValueCodecError> {
    let payload_len = u64::try_from(payload.len())
        .map_err(|_| ValueCodecError::InvalidEnvelope("payload length does not fit u64"))?;
    let raw_len = u64::try_from(raw_len)
        .map_err(|_| ValueCodecError::InvalidEnvelope("raw length does not fit u64"))?;

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

        let decoded: Sample = decode_default(&bytes).unwrap();
        assert_eq!(
            decoded,
            Sample {
                title: "legacy".into()
            }
        );
    }

    #[test]
    fn new_payloads_always_have_envelope_magic() {
        let bytes = encode_default(&Sample {
            title: "new".into(),
        })
        .unwrap();

        assert!(has_envelope_magic(&bytes));
    }

    #[test]
    fn compressed_payload_must_decode_to_nested_plain_envelope() {
        let bytes = encode_default(&Sample {
            title: "x".repeat(4096),
        })
        .unwrap();

        let payload = decode_default_bytes(&bytes).unwrap();
        let decoded: Sample = postcard::from_bytes(payload.as_ref()).unwrap();
        assert_eq!(decoded.title, "x".repeat(4096));
    }
}
