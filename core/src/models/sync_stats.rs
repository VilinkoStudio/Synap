use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    kvstore::{KvReader, KvStore},
    types::BlockId,
};

const SYNC_STATS_STORE: KvStore<BlockId, SyncStatsRecord> = KvStore::new("SyncSessionStats");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncSessionRole {
    Initiator,
    Listener,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncSessionStatus {
    Completed,
    PendingTrust,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncStatsRecord {
    pub id: Uuid,
    pub role: SyncSessionRole,
    pub status: SyncSessionStatus,
    pub peer_key_id: Option<Uuid>,
    pub peer_public_key: Option<Vec<u8>>,
    pub peer_fingerprint: Option<Vec<u8>>,
    pub peer_label: Option<String>,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub records_sent: u64,
    pub records_received: u64,
    pub records_applied: u64,
    pub records_skipped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

pub struct SyncStatsReader<'a> {
    records: KvReader<BlockId, SyncStatsRecord>,
    _marker: std::marker::PhantomData<&'a ReadTransaction>,
}

impl<'a> SyncStatsReader<'a> {
    pub fn new(tx: &'a ReadTransaction) -> Result<Self, redb::Error> {
        Ok(Self {
            records: SYNC_STATS_STORE.reader(tx)?,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn get(&self, id: &Uuid) -> Result<Option<SyncStatsRecord>, redb::Error> {
        self.records.get(id.as_bytes())
    }

    pub fn all(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<SyncStatsRecord, redb::StorageError>> + '_,
        redb::StorageError,
    > {
        let iter = self.records.iter()?;
        Ok(iter.map(|item| item.map(|(_, record)| record)))
    }
}

pub struct SyncStatsWriter<'a> {
    tx: &'a WriteTransaction,
}

impl<'a> SyncStatsWriter<'a> {
    pub fn new(tx: &'a WriteTransaction) -> Self {
        Self { tx }
    }

    pub fn init_schema(tx: &WriteTransaction) -> Result<(), redb::Error> {
        SYNC_STATS_STORE.init_table(tx)?;
        Ok(())
    }

    pub fn put(&self, record: &SyncStatsRecord) -> Result<(), redb::Error> {
        SYNC_STATS_STORE.put(self.tx, record.id.as_bytes(), record)
    }

    pub fn get(&self, id: &Uuid) -> Result<Option<SyncStatsRecord>, redb::Error> {
        SYNC_STATS_STORE.get_in_write(self.tx, id.as_bytes())
    }

    pub fn delete(&self, id: &Uuid) -> Result<bool, redb::Error> {
        SYNC_STATS_STORE.delete(self.tx, id.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    fn temp_db() -> Database {
        let file = NamedTempFile::new().unwrap();
        Database::create(file.path()).unwrap()
    }

    #[test]
    fn test_sync_stats_schema_and_roundtrip() {
        let db = temp_db();

        let write_tx = db.begin_write().unwrap();
        SyncStatsWriter::init_schema(&write_tx).unwrap();

        let writer = SyncStatsWriter::new(&write_tx);
        let record = SyncStatsRecord {
            id: Uuid::from_u128(1),
            role: SyncSessionRole::Initiator,
            status: SyncSessionStatus::Completed,
            peer_key_id: Some(Uuid::from_u128(2)),
            peer_public_key: Some(vec![5; 32]),
            peer_fingerprint: Some(vec![1, 2, 3, 4]),
            peer_label: Some("peer-b".into()),
            started_at_ms: 100,
            finished_at_ms: 240,
            records_sent: 3,
            records_received: 4,
            records_applied: 2,
            records_skipped: 1,
            bytes_sent: 512,
            bytes_received: 768,
            duration_ms: 140,
            error_message: None,
        };

        writer.put(&record).unwrap();
        assert_eq!(writer.get(&record.id).unwrap(), Some(record.clone()));
        write_tx.commit().unwrap();

        let read_tx = db.begin_read().unwrap();
        let reader = SyncStatsReader::new(&read_tx).unwrap();

        assert_eq!(reader.get(&record.id).unwrap(), Some(record.clone()));

        let all = reader
            .all()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(all, vec![record]);
    }

    #[test]
    fn test_sync_stats_delete() {
        let db = temp_db();

        let write_tx = db.begin_write().unwrap();
        SyncStatsWriter::init_schema(&write_tx).unwrap();
        let writer = SyncStatsWriter::new(&write_tx);
        let record_id = Uuid::from_u128(42);

        writer
            .put(&SyncStatsRecord {
                id: record_id,
                role: SyncSessionRole::Listener,
                status: SyncSessionStatus::PendingTrust,
                peer_key_id: None,
                peer_public_key: Some(vec![7; 32]),
                peer_fingerprint: Some(vec![9, 9, 9]),
                peer_label: Some("pending-peer".into()),
                started_at_ms: 1,
                finished_at_ms: 2,
                records_sent: 0,
                records_received: 0,
                records_applied: 0,
                records_skipped: 0,
                bytes_sent: 32,
                bytes_received: 64,
                duration_ms: 1,
                error_message: Some("pending trust".into()),
            })
            .unwrap();

        assert!(writer.delete(&record_id).unwrap());
        assert!(!writer.delete(&record_id).unwrap());
        write_tx.commit().unwrap();

        let read_tx = db.begin_read().unwrap();
        let reader = SyncStatsReader::new(&read_tx).unwrap();
        assert!(reader.get(&record_id).unwrap().is_none());
    }
}
