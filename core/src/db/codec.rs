use std::borrow::Cow;

use serde::{de::DeserializeOwned, Serialize};

pub(crate) use crate::envelope::{EnvelopeError as ValueCodecError, ENVELOPE_MAGIC};

pub(crate) fn encode_default<V: Serialize>(value: &V) -> Result<Vec<u8>, ValueCodecError> {
    let payload = postcard::to_allocvec(value).map_err(|_| {
        ValueCodecError::InvalidEnvelope("failed to serialize value into postcard bytes")
    })?;
    crate::envelope::encode_bytes(&payload, &crate::envelope::EnvelopeConfig::DEFAULT)
}

pub(crate) fn decode_default<V: DeserializeOwned>(bytes: &[u8]) -> Result<V, ValueCodecError> {
    let payload = crate::envelope::decode_bytes(bytes, &crate::envelope::EnvelopeConfig::DEFAULT)?;
    postcard::from_bytes(payload.as_ref())
        .map_err(|_| ValueCodecError::InvalidEnvelope("failed to deserialize postcard payload"))
}

pub(crate) fn decode_default_bytes(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ValueCodecError> {
    crate::envelope::decode_bytes(bytes, &crate::envelope::EnvelopeConfig::DEFAULT)
}

pub(crate) fn has_envelope_magic(bytes: &[u8]) -> bool {
    bytes.len() >= ENVELOPE_MAGIC.len() && bytes[..ENVELOPE_MAGIC.len()] == ENVELOPE_MAGIC
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
