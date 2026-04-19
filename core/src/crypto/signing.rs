use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use uuid::Uuid;

use crate::models::crypto::{
    CryptoReader, CryptoWriter, KeyMetadataRecord, KeyPurpose, KeyStatus, KeyVisibility,
    SensitiveKeyRecord,
};

// 为本地数据库的签名身份预留固定主键。
// 这把 key 用来做应用层签名，不和 X25519 的协商身份混用。
const LOCAL_SIGNING_KEY_ID: Uuid = Uuid::from_u128(0x0f4db6f5_f369_4f11_a6be_36a23a3db011);
const LOCAL_SIGNING_SECRET_ID: Uuid = Uuid::from_u128(0x0f4db6f5_f369_4f11_a6be_36a23a3db012);

pub fn local_signing_key_id() -> Uuid {
    LOCAL_SIGNING_KEY_ID
}

pub fn local_signing_secret_id() -> Uuid {
    LOCAL_SIGNING_SECRET_ID
}

/// 确保当前数据库存在一组本地 Ed25519 签名身份。
///
/// 这组密钥用于应用层签名和验签，不承担密钥协商职责。
pub fn ensure_local_signing_identity(
    writer: &CryptoWriter<'_>,
) -> Result<KeyMetadataRecord, redb::Error> {
    let metadata = writer.get_metadata(LOCAL_SIGNING_KEY_ID)?;
    let secret = writer.get_sensitive(LOCAL_SIGNING_SECRET_ID)?;

    if let (Some(metadata), Some(secret)) = (metadata, secret) {
        validate_local_signing_identity(&metadata, &secret)?;
        return Ok(metadata);
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let metadata = KeyMetadataRecord {
        id: LOCAL_SIGNING_KEY_ID,
        name: Some("local database signing identity".into()),
        note: None,
        kind: "keypair".into(),
        algorithm: "ed25519".into(),
        purpose: KeyPurpose::Identity,
        visibility: KeyVisibility::Secret,
        status: KeyStatus::Active,
        fingerprint: None,
        secret_ref: Some(LOCAL_SIGNING_SECRET_ID),
        kek_ref: None,
        public_key: Some(verifying_key.to_bytes().to_vec()),
    };

    let sensitive = SensitiveKeyRecord {
        id: LOCAL_SIGNING_SECRET_ID,
        owner_key_id: LOCAL_SIGNING_KEY_ID,
        encoding: "ed25519-secret-key-v1".into(),
        private_key: signing_key.to_bytes().to_vec(),
    };

    writer.put_metadata(&metadata)?;
    writer.put_sensitive(&sensitive)?;

    Ok(metadata)
}

/// 读取当前数据库的 Ed25519 公钥。
pub fn local_signing_public_key(
    reader: &CryptoReader<'_>,
) -> Result<Option<[u8; 32]>, redb::Error> {
    let Some(metadata) = reader.get_metadata(&LOCAL_SIGNING_KEY_ID)? else {
        return Ok(None);
    };
    let Some(public_key) = metadata.public_key else {
        return Err(invalid_signing_record(
            "local signing metadata missing public key",
        ));
    };
    let bytes: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_signing_record("local signing public key must be 32 bytes"))?;
    Ok(Some(bytes))
}

/// 使用本地 Ed25519 私钥对任意字节序列签名。
pub fn sign_with_local_identity(
    reader: &CryptoReader<'_>,
    message: &[u8],
) -> Result<Option<[u8; 64]>, redb::Error> {
    let Some(signing_key) = local_signing_private_key(reader)? else {
        return Ok(None);
    };
    Ok(Some(signing_key.sign(message).to_bytes()))
}

/// 用给定 Ed25519 公钥验签。
///
/// 这里只处理纯签名校验，不掺杂 trust 决策。
pub fn verify_signed_bytes(public_key: [u8; 32], message: &[u8], signature: [u8; 64]) -> bool {
    let verifying_key = match VerifyingKey::from_bytes(&public_key) {
        Ok(key) => key,
        Err(_) => return false,
    };
    let signature = Signature::from_bytes(&signature);
    verifying_key.verify(message, &signature).is_ok()
}

fn local_signing_private_key(reader: &CryptoReader<'_>) -> Result<Option<SigningKey>, redb::Error> {
    let Some(secret) = reader.get_sensitive(&LOCAL_SIGNING_SECRET_ID)? else {
        return Ok(None);
    };
    if secret.encoding != "ed25519-secret-key-v1" {
        return Err(invalid_signing_record(
            "local signing secret encoding must be ed25519-secret-key-v1",
        ));
    }
    let bytes: [u8; 32] = secret
        .private_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_signing_record("local signing secret must be 32 bytes"))?;
    Ok(Some(SigningKey::from_bytes(&bytes)))
}

fn validate_local_signing_identity(
    metadata: &KeyMetadataRecord,
    secret: &SensitiveKeyRecord,
) -> Result<(), redb::Error> {
    if metadata.algorithm != "ed25519" {
        return Err(invalid_signing_record(
            "local signing algorithm must be ed25519",
        ));
    }
    if metadata.secret_ref != Some(LOCAL_SIGNING_SECRET_ID) {
        return Err(invalid_signing_record(
            "local signing metadata must point to reserved secret record",
        ));
    }

    let public_key = metadata
        .public_key
        .as_ref()
        .ok_or_else(|| invalid_signing_record("local signing metadata missing public key"))?;
    let public_key: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_signing_record("local signing public key must be 32 bytes"))?;

    if secret.owner_key_id != LOCAL_SIGNING_KEY_ID {
        return Err(invalid_signing_record(
            "local signing secret owner must point to reserved signing record",
        ));
    }
    if secret.encoding != "ed25519-secret-key-v1" {
        return Err(invalid_signing_record(
            "local signing secret encoding must be ed25519-secret-key-v1",
        ));
    }
    let private_key: [u8; 32] = secret
        .private_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_signing_record("local signing secret must be 32 bytes"))?;

    let signing_key = SigningKey::from_bytes(&private_key);
    if signing_key.verifying_key().to_bytes() != public_key {
        return Err(invalid_signing_record(
            "local signing secret/public key material does not match",
        ));
    }

    Ok(())
}

fn invalid_signing_record(message: &'static str) -> redb::Error {
    redb::Error::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    #[test]
    fn test_ensure_local_signing_identity_is_idempotent() {
        let db = temp_db();

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        let created = ensure_local_signing_identity(&writer).unwrap();
        let created_public = created.public_key.clone().unwrap();
        wtx.commit().unwrap();

        let wtx = db.begin_write().unwrap();
        let writer = CryptoWriter::new(&wtx);
        let ensured = ensure_local_signing_identity(&writer).unwrap();
        wtx.commit().unwrap();

        assert_eq!(ensured.id, LOCAL_SIGNING_KEY_ID);
        assert_eq!(ensured.public_key, Some(created_public));
    }

    #[test]
    fn test_local_signing_identity_can_sign_and_verify() {
        let db = temp_db();
        let message = b"synap signed payload";

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        ensure_local_signing_identity(&writer).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let public_key = local_signing_public_key(&reader).unwrap().unwrap();
        let signature = sign_with_local_identity(&reader, message).unwrap().unwrap();

        assert!(verify_signed_bytes(public_key, message, signature));
        assert!(!verify_signed_bytes(public_key, b"tampered", signature));
    }
}
