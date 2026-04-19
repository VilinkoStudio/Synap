use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    kvstore::{KvReader, KvStore},
    types::BlockId,
};

const KEY_METADATA_STORE: KvStore<BlockId, KeyMetadataRecord> = KvStore::new("CryptoKeyMetadata");
const SENSITIVE_KEY_STORE: KvStore<BlockId, SensitiveKeyRecord> =
    KvStore::new("CryptoSensitiveKeys");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyVisibility {
    Public,
    Secret,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyStatus {
    Pending,
    Active,
    Retired,
    Revoked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyPurpose {
    Identity,
    KeyAgreement,
    TrustAnchor,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyMetadataRecord {
    pub id: Uuid,
    pub name: Option<String>,
    pub note: Option<String>,
    pub kind: String,
    pub algorithm: String,
    pub purpose: KeyPurpose,
    pub visibility: KeyVisibility,
    pub status: KeyStatus,
    pub fingerprint: Option<Vec<u8>>,
    pub secret_ref: Option<Uuid>,
    pub kek_ref: Option<Uuid>,
    pub public_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveKeyRecord {
    pub id: Uuid,
    pub owner_key_id: Uuid,
    pub encoding: String,
    pub private_key: Vec<u8>,
}

pub struct CryptoReader<'a> {
    metadata: KvReader<BlockId, KeyMetadataRecord>,
    sensitive: KvReader<BlockId, SensitiveKeyRecord>,
    _marker: std::marker::PhantomData<&'a ReadTransaction>,
}

impl<'a> CryptoReader<'a> {
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            metadata: KEY_METADATA_STORE.reader(tx)?,
            sensitive: SENSITIVE_KEY_STORE.reader(tx)?,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn get_metadata(&self, id: &Uuid) -> Result<Option<KeyMetadataRecord>, redb::Error> {
        self.metadata.get(id.as_bytes())
    }

    pub fn get_sensitive(&self, id: &Uuid) -> Result<Option<SensitiveKeyRecord>, redb::Error> {
        self.sensitive.get(id.as_bytes())
    }

    pub fn all_metadata(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<KeyMetadataRecord, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self.metadata.iter()?;
        Ok(iter.map(|item| item.map(|(_, record)| record)))
    }

    pub fn all_sensitive(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<SensitiveKeyRecord, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self.sensitive.iter()?;
        Ok(iter.map(|item| item.map(|(_, record)| record)))
    }
}

pub struct CryptoWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> CryptoWriter<'a> {
    pub fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        KEY_METADATA_STORE.init_table(tx)?;
        SENSITIVE_KEY_STORE.init_table(tx)?;
        Ok(())
    }

    pub fn put_metadata(&self, record: &KeyMetadataRecord) -> Result<(), redb::Error> {
        KEY_METADATA_STORE.put(self.tx, record.id.as_bytes(), record)
    }

    pub fn put_sensitive(&self, record: &SensitiveKeyRecord) -> Result<(), redb::Error> {
        SENSITIVE_KEY_STORE.put(self.tx, record.id.as_bytes(), record)
    }

    pub fn get_metadata(&self, id: Uuid) -> Result<Option<KeyMetadataRecord>, redb::Error> {
        KEY_METADATA_STORE.get_in_write(self.tx, id.as_bytes())
    }

    pub fn get_sensitive(&self, id: Uuid) -> Result<Option<SensitiveKeyRecord>, redb::Error> {
        SENSITIVE_KEY_STORE.get_in_write(self.tx, id.as_bytes())
    }

    pub fn delete_metadata(&self, id: &Uuid) -> Result<bool, redb::Error> {
        KEY_METADATA_STORE.delete(self.tx, id.as_bytes())
    }

    pub fn delete_sensitive(&self, id: &Uuid) -> Result<bool, redb::Error> {
        SENSITIVE_KEY_STORE.delete(self.tx, id.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    #[test]
    fn test_crypto_schema_and_roundtrip() {
        let db = temp_db();

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();

        let writer = CryptoWriter::new(&wtx);
        let key_id = Uuid::from_u128(1);
        let secret_id = Uuid::from_u128(2);
        let kek_id = Uuid::from_u128(3);

        writer
            .put_metadata(&KeyMetadataRecord {
                id: key_id,
                name: Some("device identity".into()),
                note: Some("for tests".into()),
                kind: "keypair".into(),
                algorithm: "ed25519".into(),
                purpose: KeyPurpose::Identity,
                visibility: KeyVisibility::Secret,
                status: KeyStatus::Active,
                fingerprint: Some(vec![1, 2, 3, 4]),
                secret_ref: Some(secret_id),
                kek_ref: Some(kek_id),
                public_key: Some(vec![5, 6, 7, 8]),
            })
            .unwrap();

        writer
            .put_sensitive(&SensitiveKeyRecord {
                id: secret_id,
                owner_key_id: key_id,
                encoding: "pkcs8".into(),
                private_key: vec![7, 8, 9],
            })
            .unwrap();

        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();

        let metadata = reader.get_metadata(&key_id).unwrap().unwrap();
        assert_eq!(metadata.algorithm, "ed25519");
        assert_eq!(metadata.secret_ref, Some(secret_id));
        assert_eq!(metadata.public_key, Some(vec![5, 6, 7, 8]));
        assert_eq!(metadata.note.as_deref(), Some("for tests"));

        let sensitive = reader.get_sensitive(&secret_id).unwrap().unwrap();
        assert_eq!(sensitive.owner_key_id, key_id);
        assert_eq!(sensitive.encoding, "pkcs8");
        assert_eq!(sensitive.private_key, vec![7, 8, 9]);
    }
}
