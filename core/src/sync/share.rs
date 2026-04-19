use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    envelope::{self, EnvelopeConfig},
    models::note::NoteRecord,
};

pub const SHARE_VERSION: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SharePackage {
    pub(crate) version: u8,
    pub(crate) records: Vec<NoteRecord>,
}

impl SharePackage {
    pub(crate) fn new(records: Vec<NoteRecord>) -> Self {
        Self {
            version: SHARE_VERSION,
            records,
        }
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, ShareError> {
        let payload = postcard::to_allocvec(self).map_err(|_| {
            envelope::EnvelopeError::InvalidEnvelope("share postcard encode failed")
        })?;
        envelope::encode_bytes(&payload, &EnvelopeConfig::DEFAULT).map_err(Into::into)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, ShareError> {
        let payload =
            envelope::decode_bytes(bytes, &EnvelopeConfig::DEFAULT).map_err(ShareError::from)?;
        let package: Self = postcard::from_bytes(payload.as_ref())
            .map_err(|_| envelope::EnvelopeError::InvalidEnvelope("share postcard decode failed"))
            .map_err(ShareError::from)?;
        if package.version != SHARE_VERSION {
            return Err(ShareError::VersionMismatch {
                expected: SHARE_VERSION,
                got: package.version,
            });
        }
        Ok(package)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareStats {
    pub records: usize,
    pub applied: usize,
    pub bytes: usize,
    pub duration_ms: u64,
}

impl Default for ShareStats {
    fn default() -> Self {
        Self {
            records: 0,
            applied: 0,
            bytes: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Error)]
pub enum ShareError {
    #[error(transparent)]
    Envelope(#[from] crate::envelope::EnvelopeError),

    #[error("unsupported share version: expected {expected}, got {got}")]
    VersionMismatch { expected: u8, got: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_wraps_package_in_envelope() {
        let bytes = SharePackage::new(Vec::new()).encode().unwrap();
        assert!(bytes.starts_with(&envelope::ENVELOPE_MAGIC));
    }

    #[test]
    fn decode_accepts_legacy_plain_payload() {
        let package = SharePackage::new(Vec::new());
        let bytes = postcard::to_allocvec(&package).unwrap();

        assert_eq!(SharePackage::decode(&bytes).unwrap(), package);
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let payload = postcard::to_allocvec(&SharePackage {
            version: SHARE_VERSION + 1,
            records: Vec::new(),
        })
        .unwrap();
        let bytes = envelope::encode_bytes(&payload, &EnvelopeConfig::DEFAULT).unwrap();

        assert!(matches!(
            SharePackage::decode(&bytes),
            Err(ShareError::VersionMismatch { .. })
        ));
    }
}
