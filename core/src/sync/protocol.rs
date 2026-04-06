use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{envelope, models::note::NoteRecord};

pub const PROTOCOL_VERSION: u8 = 1;
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// A transport-agnostic duplex byte channel for sync.
///
/// Frontends are expected to bridge their own TCP/Bluetooth/WebRTC/etc.
/// implementation to this trait. The core never depends on a real network stack.
pub trait SyncChannel: Read + Write + Send {
    fn close(&mut self) -> io::Result<()> {
        self.flush()
    }
}

impl<T> SyncChannel for T where T: Read + Write + Send {}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub max_records_per_message: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_records_per_message: 256,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncStats {
    pub records_sent: usize,
    pub records_received: usize,
    pub records_applied: usize,
    pub records_skipped: usize,
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SyncRecordId(pub Uuid);

impl SyncRecordId {
    pub fn for_record(record: &NoteRecord) -> Result<Self, postcard::Error> {
        Ok(Self(record.sync_id()?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncBucketSummary {
    pub bucket: u8,
    pub record_count: usize,
    pub digest: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SyncBucketEntry {
    pub bucket: u8,
    pub record_id: SyncRecordId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncMessage {
    Hello { version: u8 },
    Manifest { buckets: Vec<SyncBucketSummary> },
    BucketEntries { entries: Vec<SyncBucketEntry> },
    BucketEntriesDone,
    Records { records: Vec<NoteRecord> },
    RecordsDone,
    Done,
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("database error: {0}")]
    Db(#[from] redb::Error),

    #[error("transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("service error: {0}")]
    Service(#[from] crate::error::ServiceError),

    #[error("record encoding error: {0}")]
    Encode(#[from] postcard::Error),

    #[error("protocol version mismatch: local={local}, remote={remote}")]
    ProtocolVersionMismatch { local: u8, remote: u8 },

    #[error("unexpected message: expected {expected}, got {got:?}")]
    UnexpectedMessage {
        expected: &'static str,
        got: SyncMessage,
    },

    #[error("sync record id collision: {record_id:?}")]
    RecordIdCollision { record_id: SyncRecordId },

    #[error("received unexpected sync record: {record_id:?}")]
    UnexpectedRecord { record_id: SyncRecordId },

    #[error("received unexpected sync bucket: {bucket}")]
    UnexpectedBucket { bucket: u8 },

    #[error("invalid sync manifest: {0}")]
    InvalidManifest(String),

    #[error("sync bucket entry does not match bucket prefix: bucket={bucket}, record_id={record_id:?}")]
    BucketEntryMismatch { bucket: u8, record_id: SyncRecordId },
}

pub(crate) struct FrameCodec;

impl FrameCodec {
    pub(crate) fn write_message(
        channel: &mut impl SyncChannel,
        message: &SyncMessage,
    ) -> Result<usize, SyncError> {
        let payload = envelope::encode_postcard(message).map_err(io::Error::from)?;

        if payload.len() > MAX_FRAME_SIZE {
            return Err(SyncError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sync frame too large",
            )));
        }

        let len = payload.len() as u32;
        channel.write_all(&len.to_be_bytes())?;
        channel.write_all(&payload)?;

        Ok(payload.len() + 4)
    }

    pub(crate) fn read_message(
        channel: &mut impl SyncChannel,
    ) -> Result<(SyncMessage, usize), SyncError> {
        let mut len_bytes = [0_u8; 4];
        channel.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > MAX_FRAME_SIZE {
            return Err(SyncError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "sync frame too large",
            )));
        }

        let mut payload = vec![0_u8; len];
        channel.read_exact(&mut payload)?;
        let message = envelope::decode_postcard(&payload).map_err(io::Error::from)?;

        Ok((message, len + 4))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn write_message_wraps_payload_in_envelope() {
        let mut channel = Cursor::new(Vec::new());
        let written = FrameCodec::write_message(
            &mut channel,
            &SyncMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .unwrap();
        let bytes = channel.into_inner();

        assert_eq!(written, bytes.len());
        assert!(envelope::has_envelope_magic(&bytes[4..]));
    }

    #[test]
    fn read_message_accepts_legacy_plain_payload() {
        let message = SyncMessage::Done;
        let payload = postcard::to_allocvec(&message).unwrap();

        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);

        let mut channel = Cursor::new(bytes);
        let (decoded, read) = FrameCodec::read_message(&mut channel).unwrap();

        assert_eq!(decoded, message);
        assert_eq!(read, payload.len() + 4);
    }
}
