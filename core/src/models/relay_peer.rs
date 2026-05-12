use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{
        kvstore::{KvReader, KvStore},
        types::BlockId,
    },
    sync::RelayInventory,
};

const RELAY_PEER_STORE: KvStore<BlockId, RelayPeerRecord> = KvStore::new("RelayPeers");
const RELAY_PEER_NAMESPACE: Uuid = Uuid::from_u128(0x3f0942a3_0a75_48b0_b6f4_9361bfd61d41);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayPeerRecord {
    pub id: Uuid,
    pub peer_public_key: [u8; 32],
    pub cached_inventory: RelayInventory,
    pub cached_at_ms: u64,
}

pub struct RelayPeerReader<'a> {
    records: KvReader<BlockId, RelayPeerRecord>,
    _marker: std::marker::PhantomData<&'a ReadTransaction>,
}

impl<'a> RelayPeerReader<'a> {
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            records: RELAY_PEER_STORE.reader(tx)?,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn get(&self, id: &Uuid) -> Result<Option<RelayPeerRecord>, redb::Error> {
        self.records.get(id.as_bytes())
    }

    pub fn get_by_public_key(
        &self,
        peer_public_key: &[u8; 32],
    ) -> Result<Option<RelayPeerRecord>, redb::Error> {
        self.get(&relay_peer_id(peer_public_key))
    }

    pub fn all(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<RelayPeerRecord, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self.records.iter()?;
        Ok(iter.map(|item| item.map(|(_, record)| record)))
    }
}

pub struct RelayPeerWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> RelayPeerWriter<'a> {
    pub fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        RELAY_PEER_STORE.init_table(tx)?;
        Ok(())
    }

    pub fn put(&self, record: &RelayPeerRecord) -> Result<(), redb::Error> {
        RELAY_PEER_STORE.put(self.tx, record.id.as_bytes(), record)
    }

    pub fn put_cached_inventory(
        &self,
        peer_public_key: [u8; 32],
        cached_inventory: RelayInventory,
        cached_at_ms: u64,
    ) -> Result<RelayPeerRecord, redb::Error> {
        let record = RelayPeerRecord {
            id: relay_peer_id(&peer_public_key),
            peer_public_key,
            cached_inventory,
            cached_at_ms,
        };
        self.put(&record)?;
        Ok(record)
    }

    pub fn get(&self, id: &Uuid) -> Result<Option<RelayPeerRecord>, redb::Error> {
        RELAY_PEER_STORE.get_in_write(self.tx, id.as_bytes())
    }

    pub fn get_by_public_key(
        &self,
        peer_public_key: &[u8; 32],
    ) -> Result<Option<RelayPeerRecord>, redb::Error> {
        self.get(&relay_peer_id(peer_public_key))
    }

    pub fn delete(&self, id: &Uuid) -> Result<bool, redb::Error> {
        RELAY_PEER_STORE.delete(self.tx, id.as_bytes())
    }

    pub fn delete_by_public_key(&self, peer_public_key: &[u8; 32]) -> Result<bool, redb::Error> {
        self.delete(&relay_peer_id(peer_public_key))
    }
}

pub fn relay_peer_id(peer_public_key: &[u8; 32]) -> Uuid {
    Uuid::new_v5(&RELAY_PEER_NAMESPACE, peer_public_key)
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    use super::*;
    use crate::sync::{RelayInventory, RelayRecordDescriptor, SyncRecordId};

    fn temp_db() -> Database {
        let file = NamedTempFile::new().unwrap();
        Database::create(file.path()).unwrap()
    }

    fn sample_inventory() -> RelayInventory {
        RelayInventory {
            version: RelayInventory::VERSION,
            records: vec![RelayRecordDescriptor {
                root_note_id: Uuid::from_u128(11),
                sync_id: SyncRecordId(Uuid::from_u128(22)),
            }],
        }
    }

    #[test]
    fn relay_peer_cache_roundtrip_by_public_key() {
        let db = temp_db();
        let peer_public_key = [7u8; 32];

        let write_tx = db.begin_write().unwrap();
        RelayPeerWriter::init_schema(&write_tx).unwrap();
        let writer = RelayPeerWriter::new(&write_tx);
        let stored = writer
            .put_cached_inventory(peer_public_key, sample_inventory(), 1234)
            .unwrap();
        assert_eq!(
            writer.get_by_public_key(&peer_public_key).unwrap(),
            Some(stored.clone())
        );
        write_tx.commit().unwrap();

        let read_tx = db.begin_read().unwrap();
        let reader = RelayPeerReader::new(&read_tx).unwrap();
        assert_eq!(
            reader.get_by_public_key(&peer_public_key).unwrap(),
            Some(stored.clone())
        );

        let all = reader
            .all()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(all, vec![stored]);
    }

    #[test]
    fn relay_peer_delete_by_public_key_removes_cached_inventory() {
        let db = temp_db();
        let peer_public_key = [8u8; 32];

        let write_tx = db.begin_write().unwrap();
        RelayPeerWriter::init_schema(&write_tx).unwrap();
        let writer = RelayPeerWriter::new(&write_tx);
        writer
            .put_cached_inventory(peer_public_key, sample_inventory(), 5678)
            .unwrap();

        assert!(writer.delete_by_public_key(&peer_public_key).unwrap());
        assert!(!writer.delete_by_public_key(&peer_public_key).unwrap());
        write_tx.commit().unwrap();

        let read_tx = db.begin_read().unwrap();
        let reader = RelayPeerReader::new(&read_tx).unwrap();
        assert!(reader
            .get_by_public_key(&peer_public_key)
            .unwrap()
            .is_none());
    }
}
