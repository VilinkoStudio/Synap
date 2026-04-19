use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::crypto::{
    CryptoReader, CryptoWriter, KeyMetadataRecord, KeyPurpose, KeyStatus, KeyVisibility,
};

const TRUSTED_PUBLIC_KEY_NAMESPACE: Uuid = Uuid::from_u128(0x8db35309_7fc7_49df_9e9d_9564ef4bb001);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPublicKeyRecord {
    pub id: Uuid,
    pub algorithm: String,
    pub public_key: [u8; 32],
    pub fingerprint: [u8; 32],
    pub note: Option<String>,
    pub status: KeyStatus,
}

/// 导入一个受信任的对端 Ed25519 公钥。
///
/// 这里按公钥字节生成稳定 ID，因此重复导入同一把公钥时会自然覆盖为同一条记录，
/// 便于把“导入”和“幂等 upsert”合并成一个入口。
pub fn import_trusted_public_key(
    writer: &CryptoWriter<'_>,
    public_key: [u8; 32],
    note: Option<String>,
) -> Result<TrustedPublicKeyRecord, redb::Error> {
    upsert_public_key_record(writer, public_key, note, KeyStatus::Active)
}

pub fn remember_untrusted_public_key(
    writer: &CryptoWriter<'_>,
    public_key: [u8; 32],
    note: Option<String>,
) -> Result<TrustedPublicKeyRecord, redb::Error> {
    let id = trusted_public_key_id(&public_key);
    if let Some(metadata) = writer.get_metadata(id)? {
        if metadata.purpose != KeyPurpose::TrustAnchor {
            return Err(invalid_trust_record(
                "peer public key record must use trust-anchor purpose",
            ));
        }
        return trusted_record_from_metadata(metadata);
    }

    upsert_public_key_record(writer, public_key, note, KeyStatus::Pending)
}

pub fn get_known_public_key(
    reader: &CryptoReader<'_>,
    id: Uuid,
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    let Some(metadata) = reader.get_metadata(&id)? else {
        return Ok(None);
    };
    if metadata.purpose != KeyPurpose::TrustAnchor {
        return Ok(None);
    }
    Ok(Some(trusted_record_from_metadata(metadata)?))
}

pub fn get_known_public_key_by_bytes(
    reader: &CryptoReader<'_>,
    public_key: [u8; 32],
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    get_known_public_key(reader, trusted_public_key_id(&public_key))
}

pub fn public_key_fingerprint(public_key: &[u8; 32]) -> [u8; 32] {
    Sha256::digest(public_key).into()
}

fn upsert_public_key_record(
    writer: &CryptoWriter<'_>,
    public_key: [u8; 32],
    note: Option<String>,
    status: KeyStatus,
) -> Result<TrustedPublicKeyRecord, redb::Error> {
    let id = trusted_public_key_id(&public_key);
    let fingerprint = public_key_fingerprint(&public_key);

    let metadata = KeyMetadataRecord {
        id,
        name: None,
        note,
        kind: "public-key".into(),
        algorithm: "ed25519".into(),
        purpose: KeyPurpose::TrustAnchor,
        visibility: KeyVisibility::Public,
        status,
        fingerprint: Some(fingerprint.to_vec()),
        secret_ref: None,
        kek_ref: None,
        public_key: Some(public_key.to_vec()),
    };

    writer.put_metadata(&metadata)?;
    trusted_record_from_metadata(metadata)
}

pub fn get_trusted_public_key(
    reader: &CryptoReader<'_>,
    id: Uuid,
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    let Some(record) = get_known_public_key(reader, id)? else {
        return Ok(None);
    };
    Ok((record.status == KeyStatus::Active).then_some(record))
}

pub fn list_trusted_public_keys(
    reader: &CryptoReader<'_>,
) -> Result<Vec<TrustedPublicKeyRecord>, redb::Error> {
    list_public_keys_by_status(reader, KeyStatus::Active)
}

pub fn list_known_public_keys(
    reader: &CryptoReader<'_>,
) -> Result<Vec<TrustedPublicKeyRecord>, redb::Error> {
    let iter = reader.all_metadata().map_err(redb::Error::from)?;
    let mut records = Vec::new();
    for item in iter {
        let metadata = item.map_err(redb::Error::from)?;
        if metadata.purpose != KeyPurpose::TrustAnchor {
            continue;
        }
        records.push(trusted_record_from_metadata(metadata)?);
    }
    records.sort_by_key(|record| record.id);
    Ok(records)
}

fn list_public_keys_by_status(
    reader: &CryptoReader<'_>,
    status: KeyStatus,
) -> Result<Vec<TrustedPublicKeyRecord>, redb::Error> {
    let iter = reader.all_metadata().map_err(redb::Error::from)?;
    let mut records = Vec::new();
    for item in iter {
        let metadata = item.map_err(redb::Error::from)?;
        if metadata.purpose != KeyPurpose::TrustAnchor || metadata.status != status {
            continue;
        }
        records.push(trusted_record_from_metadata(metadata)?);
    }
    records.sort_by_key(|record| record.id);
    Ok(records)
}

/// 按公钥内容查找信任记录。
///
/// 这让握手阶段可以直接拿对端 Ed25519 公钥做 trust 判定，
/// 不必要求上层提前知道内部的 UUID。
pub fn get_trusted_public_key_by_bytes(
    reader: &CryptoReader<'_>,
    public_key: [u8; 32],
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    get_trusted_public_key(reader, trusted_public_key_id(&public_key))
}

pub fn is_trusted_public_key(
    reader: &CryptoReader<'_>,
    public_key: [u8; 32],
) -> Result<bool, redb::Error> {
    Ok(get_trusted_public_key_by_bytes(reader, public_key)?.is_some())
}

pub fn update_trusted_public_key_note(
    writer: &CryptoWriter<'_>,
    id: Uuid,
    note: Option<String>,
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    let Some(mut metadata) = writer.get_metadata(id)? else {
        return Ok(None);
    };
    if metadata.purpose != KeyPurpose::TrustAnchor {
        return Ok(None);
    }

    metadata.note = note;
    writer.put_metadata(&metadata)?;
    Ok(Some(trusted_record_from_metadata(metadata)?))
}

pub fn update_trusted_public_key_status(
    writer: &CryptoWriter<'_>,
    id: Uuid,
    status: KeyStatus,
) -> Result<Option<TrustedPublicKeyRecord>, redb::Error> {
    let Some(mut metadata) = writer.get_metadata(id)? else {
        return Ok(None);
    };
    if metadata.purpose != KeyPurpose::TrustAnchor {
        return Ok(None);
    }

    metadata.status = status;
    writer.put_metadata(&metadata)?;
    Ok(Some(trusted_record_from_metadata(metadata)?))
}

pub fn delete_trusted_public_key(writer: &CryptoWriter<'_>, id: Uuid) -> Result<bool, redb::Error> {
    let Some(metadata) = writer.get_metadata(id)? else {
        return Ok(false);
    };
    if metadata.purpose != KeyPurpose::TrustAnchor {
        return Ok(false);
    }
    writer.delete_metadata(&id)
}

fn trusted_record_from_metadata(
    metadata: KeyMetadataRecord,
) -> Result<TrustedPublicKeyRecord, redb::Error> {
    if metadata.visibility != KeyVisibility::Public {
        return Err(invalid_trust_record("trusted public key must be public"));
    }
    if metadata.algorithm != "ed25519" {
        return Err(invalid_trust_record(
            "trusted public key algorithm must be ed25519",
        ));
    }

    let public_key = metadata
        .public_key
        .ok_or_else(|| invalid_trust_record("trusted public key record missing public key"))?;
    let public_key: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_trust_record("trusted public key must be 32 bytes"))?;

    let fingerprint = metadata
        .fingerprint
        .ok_or_else(|| invalid_trust_record("trusted public key record missing fingerprint"))?;
    let fingerprint: [u8; 32] = fingerprint
        .as_slice()
        .try_into()
        .map_err(|_| invalid_trust_record("trusted public key fingerprint must be 32 bytes"))?;

    Ok(TrustedPublicKeyRecord {
        id: metadata.id,
        algorithm: metadata.algorithm,
        public_key,
        fingerprint,
        note: metadata.note,
        status: metadata.status,
    })
}

fn trusted_public_key_id(public_key: &[u8; 32]) -> Uuid {
    Uuid::new_v5(&TRUSTED_PUBLIC_KEY_NAMESPACE, public_key)
}

fn invalid_trust_record(message: &'static str) -> redb::Error {
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
    use crate::models::crypto::{CryptoReader, CryptoWriter};

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    #[test]
    fn test_trusted_public_key_crud() {
        let db = temp_db();
        let public_key = [7u8; 32];

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        let imported =
            import_trusted_public_key(&writer, public_key, Some("alice".into())).unwrap();
        let updated =
            update_trusted_public_key_note(&writer, imported.id, Some("alice-laptop".into()))
                .unwrap()
                .unwrap();
        assert_eq!(updated.note.as_deref(), Some("alice-laptop"));
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let fetched = get_trusted_public_key(&reader, imported.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.public_key, public_key);
        assert_eq!(fetched.algorithm, "ed25519");
        assert_eq!(fetched.note.as_deref(), Some("alice-laptop"));
        let listed = list_trusted_public_keys(&reader).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], fetched);
        assert!(is_trusted_public_key(&reader, public_key).unwrap());
        assert_eq!(
            get_trusted_public_key_by_bytes(&reader, public_key)
                .unwrap()
                .unwrap(),
            fetched
        );
        drop(rtx);

        let wtx = db.begin_write().unwrap();
        let writer = CryptoWriter::new(&wtx);
        assert!(delete_trusted_public_key(&writer, imported.id).unwrap());
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        assert!(get_trusted_public_key(&reader, imported.id)
            .unwrap()
            .is_none());
        assert!(list_trusted_public_keys(&reader).unwrap().is_empty());
    }

    #[test]
    fn test_import_trusted_public_key_is_idempotent_for_same_key() {
        let db = temp_db();
        let public_key = [9u8; 32];

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        let first = import_trusted_public_key(&writer, public_key, Some("first".into())).unwrap();
        let second = import_trusted_public_key(&writer, public_key, Some("second".into())).unwrap();
        wtx.commit().unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.note.as_deref(), Some("second"));
    }

    #[test]
    fn test_pending_public_key_is_not_treated_as_trusted() {
        let db = temp_db();
        let public_key = [11u8; 32];

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        let pending =
            remember_untrusted_public_key(&writer, public_key, Some("unknown".into())).unwrap();
        wtx.commit().unwrap();

        assert_eq!(pending.status, KeyStatus::Pending);

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        assert!(get_trusted_public_key(&reader, pending.id)
            .unwrap()
            .is_none());
        assert!(!is_trusted_public_key(&reader, public_key).unwrap());
        let pending_list = list_known_public_keys(&reader).unwrap();
        assert_eq!(pending_list.len(), 1);
        assert_eq!(pending_list[0].public_key, public_key);
        assert_eq!(pending_list[0].status, KeyStatus::Pending);
    }
}
