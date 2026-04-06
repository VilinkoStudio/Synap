use std::io;

use serde::{Deserialize, Serialize};

use super::protocol::SyncChannel;
use crate::{envelope, models::note::NoteRecord};

pub const SHARE_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShareHeader {
    pub version: u8,
    pub record_count: usize,
    pub total_size_hint: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShareStats {
    pub records_sent: usize,
    pub bytes_sent: usize,
    pub duration_ms: u64,
}

impl Default for ShareStats {
    fn default() -> Self {
        Self {
            records_sent: 0,
            bytes_sent: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShareMessage {
    Header(ShareHeader),
    Records { records: Vec<NoteRecord> },
    Done { stats: ShareStats },
}

pub(crate) struct FrameCodec;

impl FrameCodec {
    pub(crate) fn write_message(
        channel: &mut impl SyncChannel,
        message: &ShareMessage,
    ) -> Result<usize, io::Error> {
        let payload = envelope::encode_postcard(message).map_err(io::Error::from)?;

        let len = payload.len() as u32;
        channel.write_all(&len.to_be_bytes())?;
        channel.write_all(&payload)?;
        channel.close()?;

        Ok(payload.len() + 4)
    }

    pub(crate) fn read_message(channel: &mut impl SyncChannel) -> Result<ShareMessage, io::Error> {
        let mut len_bytes = [0_u8; 4];
        channel.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut payload = vec![0_u8; len];
        channel.read_exact(&mut payload)?;
        let message = envelope::decode_postcard(&payload).map_err(io::Error::from)?;

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn write_message_wraps_payload_in_envelope() {
        let message = ShareMessage::Header(ShareHeader {
            version: SHARE_VERSION,
            record_count: 2,
            total_size_hint: 128,
        });

        let mut channel = Cursor::new(Vec::new());
        let written = FrameCodec::write_message(&mut channel, &message).unwrap();
        let bytes = channel.into_inner();

        assert_eq!(written, bytes.len());
        assert!(envelope::has_envelope_magic(&bytes[4..]));
    }

    #[test]
    fn read_message_accepts_legacy_plain_payload() {
        let message = ShareMessage::Done {
            stats: ShareStats {
                records_sent: 3,
                bytes_sent: 256,
                duration_ms: 10,
            },
        };
        let payload = postcard::to_allocvec(&message).unwrap();

        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);

        let mut channel = Cursor::new(bytes);
        assert_eq!(FrameCodec::read_message(&mut channel).unwrap(), message);
    }
}
