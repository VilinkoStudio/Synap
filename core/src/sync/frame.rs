use std::io::{self, Read, Write};

use serde::{de::DeserializeOwned, Serialize};

use crate::envelope::{self, EnvelopeConfig};

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

pub(crate) struct FrameCodec;

impl FrameCodec {
    pub(crate) fn write<T: Serialize>(
        writer: &mut impl Write,
        message: &T,
    ) -> Result<usize, io::Error> {
        let raw = postcard::to_allocvec(message).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "frame postcard encode failed")
        })?;
        let payload =
            envelope::encode_bytes(&raw, &EnvelopeConfig::DEFAULT).map_err(io::Error::from)?;

        if payload.len() > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sync frame too large",
            ));
        }

        let len = payload.len() as u32;
        writer.write_all(&len.to_be_bytes())?;
        writer.write_all(&payload)?;
        writer.flush()?;

        Ok(payload.len() + 4)
    }

    pub(crate) fn read<T: DeserializeOwned>(
        reader: &mut impl Read,
    ) -> Result<(T, usize), io::Error> {
        let mut len_bytes = [0_u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sync frame too large",
            ));
        }

        let mut payload = vec![0_u8; len];
        reader.read_exact(&mut payload)?;
        let raw =
            envelope::decode_bytes(&payload, &EnvelopeConfig::DEFAULT).map_err(io::Error::from)?;
        let message = postcard::from_bytes(raw.as_ref()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "frame postcard decode failed")
        })?;

        Ok((message, len + 4))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Sample {
        title: String,
    }

    #[test]
    fn write_wraps_payload_in_envelope() {
        let mut channel = Cursor::new(Vec::new());
        let written = FrameCodec::write(
            &mut channel,
            &Sample {
                title: "hello".into(),
            },
        )
        .unwrap();
        let bytes = channel.into_inner();

        assert_eq!(written, bytes.len());
        assert!(bytes[4..].starts_with(&envelope::ENVELOPE_MAGIC));
    }

    #[test]
    fn read_accepts_legacy_plain_payload() {
        let message = Sample {
            title: "legacy".into(),
        };
        let payload = postcard::to_allocvec(&message).unwrap();

        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);

        let mut channel = Cursor::new(bytes);
        let (decoded, read) = FrameCodec::read::<Sample>(&mut channel).unwrap();

        assert_eq!(decoded, message);
        assert_eq!(read, payload.len() + 4);
    }
}
