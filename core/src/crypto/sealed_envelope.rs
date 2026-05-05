use std::io;

use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::{
    crypto::{
        get_known_public_key_by_bytes, local_identity_private_key, local_signing_public_key,
        public_key_fingerprint, sign_with_local_identity, verify_signed_bytes,
    },
    envelope::{decode_bytes, encode_bytes, EnvelopeConfig, EnvelopeEncryptionConfig},
    models::crypto::CryptoReader,
};

use super::TrustedPublicKeyRecord;

pub const SEALED_ENVELOPE_MAGIC: [u8; 4] = *b"SKE!";

const SEALED_ENVELOPE_VERSION: u8 = 1;
const SIGNING_CONTEXT: &[u8] = b"synap.crypto.sealed-envelope.signature.v1";
const KEY_DERIVATION_CONTEXT: &[u8] = b"synap.crypto.sealed-envelope.key-derivation.v1";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeSenderIdentity {
    pub public_key: [u8; 32],
    pub fingerprint: [u8; 32],
    pub known_record: Option<TrustedPublicKeyRecord>,
}

impl EnvelopeSenderIdentity {
    pub fn is_trusted(&self) -> bool {
        self.known_record
            .as_ref()
            .is_some_and(|record| record.status == crate::models::crypto::KeyStatus::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedSealedEnvelope {
    pub sender: EnvelopeSenderIdentity,
    pub recipient_identity_public_key: [u8; 32],
    pub recipient_fingerprint: [u8; 32],
    pub encrypted_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSealedEnvelopeHeader {
    pub sender_signing_public_key: [u8; 32],
    pub sender_fingerprint: [u8; 32],
    pub recipient_identity_public_key: [u8; 32],
    pub recipient_fingerprint: [u8; 32],
    pub encrypted_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedSealedEnvelope {
    pub sender: EnvelopeSenderIdentity,
    pub recipient_identity_public_key: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SealedEnvelopeWire {
    version: u8,
    sender_signing_public_key: [u8; 32],
    recipient_identity_public_key: [u8; 32],
    ephemeral_identity_public_key: [u8; 32],
    sealed_payload: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum SealedEnvelopeError {
    #[error("database error: {0}")]
    Db(#[from] redb::Error),

    #[error("local signing identity is missing")]
    MissingLocalSigningIdentity,

    #[error("local recipient identity is missing")]
    MissingLocalRecipientIdentity,

    #[error("sealed envelope is malformed: {0}")]
    InvalidEnvelope(&'static str),

    #[error("unsupported sealed envelope version: {0}")]
    UnsupportedVersion(u8),

    #[error("sealed envelope signature is invalid")]
    InvalidSignature,

    #[error("sealed envelope recipient does not match provided private key")]
    RecipientMismatch,

    #[error(transparent)]
    Envelope(#[from] crate::envelope::EnvelopeError),
}

impl From<SealedEnvelopeError> for io::Error {
    fn from(value: SealedEnvelopeError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, value)
    }
}

pub fn seal_for_recipient(
    reader: &CryptoReader<'_>,
    recipient_identity_public_key: [u8; 32],
    payload: &[u8],
) -> Result<Vec<u8>, SealedEnvelopeError> {
    let sender_signing_public_key = local_signing_public_key(reader)?
        .ok_or(SealedEnvelopeError::MissingLocalSigningIdentity)?;

    let ephemeral_secret = StaticSecret::random();
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret).to_bytes();
    let recipient_public = X25519PublicKey::from(recipient_identity_public_key);
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_public);
    let content_key = derive_content_key(
        shared_secret.as_bytes(),
        recipient_identity_public_key,
        ephemeral_public,
    )?;

    let sealed_payload = encode_bytes(
        payload,
        &EnvelopeConfig::DEFAULT.with_encryption(EnvelopeEncryptionConfig { key: content_key }),
    )?;

    let signature_payload = signature_payload(
        sender_signing_public_key,
        recipient_identity_public_key,
        ephemeral_public,
        &sealed_payload,
    );
    let signature = sign_with_local_identity(reader, &signature_payload)?
        .ok_or(SealedEnvelopeError::MissingLocalSigningIdentity)?;

    let wire = SealedEnvelopeWire {
        version: SEALED_ENVELOPE_VERSION,
        sender_signing_public_key,
        recipient_identity_public_key,
        ephemeral_identity_public_key: ephemeral_public,
        sealed_payload,
        signature: signature.to_vec(),
    };

    let mut encoded =
        Vec::with_capacity(SEALED_ENVELOPE_MAGIC.len() + 128 + wire.sealed_payload.len());
    encoded.extend_from_slice(&SEALED_ENVELOPE_MAGIC);
    encoded.extend_from_slice(
        &postcard::to_allocvec(&wire)
            .map_err(|_| SealedEnvelopeError::InvalidEnvelope("sealed envelope encode failed"))?,
    );
    Ok(encoded)
}

pub fn inspect(
    reader: &CryptoReader<'_>,
    bytes: &[u8],
) -> Result<InspectedSealedEnvelope, SealedEnvelopeError> {
    let header = inspect_verified(bytes)?;
    let sender = load_sender_identity(reader, header.sender_signing_public_key)?;
    Ok(InspectedSealedEnvelope {
        sender,
        recipient_identity_public_key: header.recipient_identity_public_key,
        recipient_fingerprint: header.recipient_fingerprint,
        encrypted_payload_len: header.encrypted_payload_len,
    })
}

pub fn inspect_verified(bytes: &[u8]) -> Result<VerifiedSealedEnvelopeHeader, SealedEnvelopeError> {
    let wire = decode_wire(bytes)?;
    verify_wire_signature(&wire)?;

    Ok(VerifiedSealedEnvelopeHeader {
        sender_signing_public_key: wire.sender_signing_public_key,
        sender_fingerprint: public_key_fingerprint(&wire.sender_signing_public_key),
        recipient_identity_public_key: wire.recipient_identity_public_key,
        recipient_fingerprint: public_key_fingerprint(&wire.recipient_identity_public_key),
        encrypted_payload_len: wire.sealed_payload.len(),
    })
}

pub fn open_for_local_recipient(
    reader: &CryptoReader<'_>,
    bytes: &[u8],
) -> Result<OpenedSealedEnvelope, SealedEnvelopeError> {
    let recipient_private_key = local_identity_private_key(reader)?
        .ok_or(SealedEnvelopeError::MissingLocalRecipientIdentity)?;
    open_with_recipient_private_key(reader, bytes, &recipient_private_key)
}

pub fn open_with_recipient_private_key(
    reader: &CryptoReader<'_>,
    bytes: &[u8],
    recipient_private_key: &StaticSecret,
) -> Result<OpenedSealedEnvelope, SealedEnvelopeError> {
    let wire = decode_wire(bytes)?;
    verify_wire_signature(&wire)?;

    let actual_recipient_public_key = X25519PublicKey::from(recipient_private_key).to_bytes();
    if actual_recipient_public_key != wire.recipient_identity_public_key {
        return Err(SealedEnvelopeError::RecipientMismatch);
    }

    let sender = load_sender_identity(reader, wire.sender_signing_public_key)?;
    let shared_secret = recipient_private_key
        .diffie_hellman(&X25519PublicKey::from(wire.ephemeral_identity_public_key));
    let content_key = derive_content_key(
        shared_secret.as_bytes(),
        wire.recipient_identity_public_key,
        wire.ephemeral_identity_public_key,
    )?;
    let payload = decode_bytes(
        &wire.sealed_payload,
        &EnvelopeConfig::DEFAULT.with_encryption(EnvelopeEncryptionConfig { key: content_key }),
    )?
    .into_owned();

    Ok(OpenedSealedEnvelope {
        sender,
        recipient_identity_public_key: wire.recipient_identity_public_key,
        payload,
    })
}

fn decode_wire(bytes: &[u8]) -> Result<SealedEnvelopeWire, SealedEnvelopeError> {
    if bytes.len() < SEALED_ENVELOPE_MAGIC.len() {
        return Err(SealedEnvelopeError::InvalidEnvelope("truncated envelope"));
    }
    if bytes[..SEALED_ENVELOPE_MAGIC.len()] != SEALED_ENVELOPE_MAGIC {
        return Err(SealedEnvelopeError::InvalidEnvelope(
            "missing sealed envelope magic",
        ));
    }

    let wire: SealedEnvelopeWire = postcard::from_bytes(&bytes[SEALED_ENVELOPE_MAGIC.len()..])
        .map_err(|_| SealedEnvelopeError::InvalidEnvelope("sealed envelope decode failed"))?;
    if wire.version != SEALED_ENVELOPE_VERSION {
        return Err(SealedEnvelopeError::UnsupportedVersion(wire.version));
    }
    if wire.sealed_payload.is_empty() {
        return Err(SealedEnvelopeError::InvalidEnvelope(
            "sealed payload is empty",
        ));
    }
    if wire.signature.len() != 64 {
        return Err(SealedEnvelopeError::InvalidEnvelope(
            "signature must be 64 bytes",
        ));
    }
    Ok(wire)
}

fn verify_wire_signature(wire: &SealedEnvelopeWire) -> Result<(), SealedEnvelopeError> {
    let payload = signature_payload(
        wire.sender_signing_public_key,
        wire.recipient_identity_public_key,
        wire.ephemeral_identity_public_key,
        &wire.sealed_payload,
    );
    let signature: [u8; 64] = wire
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| SealedEnvelopeError::InvalidEnvelope("signature must be 64 bytes"))?;
    if !verify_signed_bytes(wire.sender_signing_public_key, &payload, signature) {
        return Err(SealedEnvelopeError::InvalidSignature);
    }
    Ok(())
}

fn load_sender_identity(
    reader: &CryptoReader<'_>,
    sender_signing_public_key: [u8; 32],
) -> Result<EnvelopeSenderIdentity, SealedEnvelopeError> {
    Ok(EnvelopeSenderIdentity {
        public_key: sender_signing_public_key,
        fingerprint: public_key_fingerprint(&sender_signing_public_key),
        known_record: get_known_public_key_by_bytes(reader, sender_signing_public_key)?,
    })
}

fn derive_content_key(
    shared_secret: &[u8; 32],
    recipient_identity_public_key: [u8; 32],
    ephemeral_identity_public_key: [u8; 32],
) -> Result<[u8; 32], SealedEnvelopeError> {
    let mut okm = [0u8; 32];
    let mut info = Vec::with_capacity(KEY_DERIVATION_CONTEXT.len() + 64);
    info.extend_from_slice(KEY_DERIVATION_CONTEXT);
    info.extend_from_slice(&recipient_identity_public_key);
    info.extend_from_slice(&ephemeral_identity_public_key);
    Hkdf::<Sha256>::new(None, shared_secret)
        .expand(&info, &mut okm)
        .map_err(|_| SealedEnvelopeError::InvalidEnvelope("hkdf output size is invalid"))?;
    Ok(okm)
}

fn envelope_aad(
    version: u8,
    sender_signing_public_key: [u8; 32],
    recipient_identity_public_key: [u8; 32],
    ephemeral_identity_public_key: [u8; 32],
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(
        SIGNING_CONTEXT.len()
            + 1
            + sender_signing_public_key.len()
            + recipient_identity_public_key.len()
            + ephemeral_identity_public_key.len(),
    );
    aad.extend_from_slice(SIGNING_CONTEXT);
    aad.push(version);
    aad.extend_from_slice(&sender_signing_public_key);
    aad.extend_from_slice(&recipient_identity_public_key);
    aad.extend_from_slice(&ephemeral_identity_public_key);
    aad
}

fn signature_payload(
    sender_signing_public_key: [u8; 32],
    recipient_identity_public_key: [u8; 32],
    ephemeral_identity_public_key: [u8; 32],
    sealed_payload: &[u8],
) -> Vec<u8> {
    let mut payload = envelope_aad(
        SEALED_ENVELOPE_VERSION,
        sender_signing_public_key,
        recipient_identity_public_key,
        ephemeral_identity_public_key,
    );
    payload.extend_from_slice(sealed_payload);
    payload
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;
    use crate::{
        crypto::{
            ensure_local_identity, ensure_local_signing_identity, import_trusted_public_key,
            local_identity_public_key, local_signing_public_key,
        },
        models::crypto::CryptoWriter,
    };

    fn temp_db() -> Database {
        let file = NamedTempFile::new().unwrap();
        Database::create(file.path()).unwrap()
    }

    fn make_sender_db() -> (Database, [u8; 32]) {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        ensure_local_signing_identity(&writer).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let public_key = local_signing_public_key(&reader).unwrap().unwrap();
        drop(rtx);

        (db, public_key)
    }

    fn make_recipient_db(trusted_sender: Option<[u8; 32]>) -> (Database, [u8; 32]) {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        ensure_local_identity(&writer).unwrap();
        if let Some(sender_public_key) = trusted_sender {
            import_trusted_public_key(&writer, sender_public_key, Some("sender".into())).unwrap();
        }
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let public_key = local_identity_public_key(&reader).unwrap().unwrap();
        drop(rtx);

        (db, public_key)
    }

    #[test]
    fn inspect_reveals_sender_identity_without_decrypting() {
        let (sender_db, sender_signing_public_key) = make_sender_db();
        let (recipient_db, recipient_identity_public_key) =
            make_recipient_db(Some(sender_signing_public_key));

        let envelope = {
            let rtx = sender_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            seal_for_recipient(&reader, recipient_identity_public_key, b"relay payload").unwrap()
        };

        let inspected = {
            let rtx = recipient_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            inspect(&reader, &envelope).unwrap()
        };

        assert_eq!(inspected.sender.public_key, sender_signing_public_key);
        assert!(inspected.sender.is_trusted());
        assert_eq!(
            inspected.recipient_identity_public_key,
            recipient_identity_public_key
        );
        assert!(inspected.encrypted_payload_len > 0);
    }

    #[test]
    fn inspect_works_for_unknown_sender_identity() {
        let (sender_db, sender_signing_public_key) = make_sender_db();
        let (recipient_db, recipient_identity_public_key) = make_recipient_db(None);

        let envelope = {
            let rtx = sender_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            seal_for_recipient(&reader, recipient_identity_public_key, b"hello").unwrap()
        };

        let inspected = {
            let rtx = recipient_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            inspect(&reader, &envelope).unwrap()
        };

        assert_eq!(inspected.sender.public_key, sender_signing_public_key);
        assert_eq!(inspected.sender.known_record, None);
        assert!(!inspected.sender.is_trusted());
    }

    #[test]
    fn open_for_local_recipient_decrypts_payload() {
        let (sender_db, sender_signing_public_key) = make_sender_db();
        let (recipient_db, recipient_identity_public_key) =
            make_recipient_db(Some(sender_signing_public_key));

        let envelope = {
            let rtx = sender_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            seal_for_recipient(
                &reader,
                recipient_identity_public_key,
                b"synap sealed envelope",
            )
            .unwrap()
        };

        let opened = {
            let rtx = recipient_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            open_for_local_recipient(&reader, &envelope).unwrap()
        };

        assert_eq!(opened.sender.public_key, sender_signing_public_key);
        assert_eq!(opened.payload, b"synap sealed envelope");
        assert_eq!(
            opened.recipient_identity_public_key,
            recipient_identity_public_key
        );
    }

    #[test]
    fn open_rejects_wrong_recipient_private_key() {
        let (sender_db, _) = make_sender_db();
        let (_recipient_db, recipient_identity_public_key) = make_recipient_db(None);
        let (wrong_recipient_db, _) = make_recipient_db(None);

        let envelope = {
            let rtx = sender_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            seal_for_recipient(&reader, recipient_identity_public_key, b"secret").unwrap()
        };

        let result = {
            let rtx = wrong_recipient_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            let wrong_private_key = local_identity_private_key(&reader).unwrap().unwrap();
            open_with_recipient_private_key(&reader, &envelope, &wrong_private_key)
        };

        assert!(matches!(
            result,
            Err(SealedEnvelopeError::RecipientMismatch)
        ));
    }

    #[test]
    fn tampered_envelope_fails_signature_validation() {
        let (sender_db, _) = make_sender_db();
        let (_recipient_db, recipient_identity_public_key) = make_recipient_db(None);

        let mut envelope = {
            let rtx = sender_db.begin_read().unwrap();
            let reader = CryptoReader::new(&rtx).unwrap();
            seal_for_recipient(&reader, recipient_identity_public_key, b"secret").unwrap()
        };
        let last = envelope.len() - 1;
        envelope[last] ^= 0x01;

        let sender_rtx = sender_db.begin_read().unwrap();
        let sender_reader = CryptoReader::new(&sender_rtx).unwrap();
        assert!(matches!(
            inspect(&sender_reader, &envelope),
            Err(SealedEnvelopeError::InvalidSignature)
        ));
    }
}
