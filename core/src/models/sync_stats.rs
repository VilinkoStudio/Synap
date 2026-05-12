use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    kvstore::{KvReader, KvStore},
    types::BlockId,
};

const SYNC_STATS_STORE: KvStore<BlockId, SyncStatsRecord> = KvStore::new("SyncSessionStatsV2");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncSessionRole {
    Initiator,
    Listener,
    RelayFetch,
    RelayPush,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncSessionStatus {
    Completed,
    PendingTrust,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncTransportKind {
    Direct,
    RelayFetch { relay_url: String },
    RelayPush { relay_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncStatsRecord {
    pub id: Uuid,
    pub role: SyncSessionRole,
    pub status: SyncSessionStatus,
    pub peer_public_key: [u8; 32],
    pub transport: SyncTransportKind,
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

    pub fn recent(&self, limit: usize) -> Result<Vec<SyncStatsRecord>, redb::Error> {
        let mut records = self
            .all()
            .map_err(redb::Error::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(redb::Error::from)?;
        sort_recent_records(&mut records);
        records.truncate(limit);
        Ok(records)
    }

    pub fn recent_for_peer(
        &self,
        peer_public_key: [u8; 32],
        limit: usize,
    ) -> Result<Vec<SyncStatsRecord>, redb::Error> {
        let mut records = self
            .all()
            .map_err(redb::Error::from)?
            .filter_map(|item| match item {
                Ok(record) if record.peer_public_key == peer_public_key => Some(Ok(record)),
                Ok(_) => None,
                Err(err) => Some(Err(redb::Error::from(err))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        sort_recent_records(&mut records);
        records.truncate(limit);
        Ok(records)
    }

    pub fn grouped_by_peer(
        &self,
        peers_limit: usize,
        sessions_per_peer: usize,
    ) -> Result<Vec<PeerSyncStatsRecord>, redb::Error> {
        let mut records = self.recent(usize::MAX)?;
        let mut groups = Vec::<PeerSyncStatsRecord>::new();

        for record in records.drain(..) {
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.peer_public_key == record.peer_public_key)
            {
                if group.recent_sessions.len() < sessions_per_peer {
                    group.recent_sessions.push(record);
                }
                continue;
            }

            if groups.len() >= peers_limit {
                continue;
            }
            groups.push(PeerSyncStatsRecord {
                peer_public_key: record.peer_public_key,
                recent_sessions: vec![record],
            });
        }

        Ok(groups)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSyncStatsRecord {
    pub peer_public_key: [u8; 32],
    pub recent_sessions: Vec<SyncStatsRecord>,
}

fn sort_recent_records(records: &mut [SyncStatsRecord]) {
    records.sort_by(|left, right| {
        right
            .finished_at_ms
            .cmp(&left.finished_at_ms)
            .then_with(|| right.started_at_ms.cmp(&left.started_at_ms))
            .then_with(|| right.id.cmp(&left.id))
    });
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
            peer_public_key: [5; 32],
            transport: SyncTransportKind::Direct,
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
                peer_public_key: [7; 32],
                transport: SyncTransportKind::Direct,
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

    #[test]
    fn test_sync_stats_groups_recent_sessions_by_peer() {
        let db = temp_db();

        let write_tx = db.begin_write().unwrap();
        SyncStatsWriter::init_schema(&write_tx).unwrap();
        let writer = SyncStatsWriter::new(&write_tx);

        for (idx, peer, finished_at_ms) in [
            (1, [1; 32], 100),
            (2, [2; 32], 300),
            (3, [1; 32], 200),
            (4, [1; 32], 50),
        ] {
            writer
                .put(&SyncStatsRecord {
                    id: Uuid::from_u128(idx),
                    role: SyncSessionRole::Initiator,
                    status: SyncSessionStatus::Completed,
                    peer_public_key: peer,
                    transport: SyncTransportKind::Direct,
                    started_at_ms: finished_at_ms - 10,
                    finished_at_ms,
                    records_sent: 0,
                    records_received: 0,
                    records_applied: 0,
                    records_skipped: 0,
                    bytes_sent: 0,
                    bytes_received: 0,
                    duration_ms: 10,
                    error_message: None,
                })
                .unwrap();
        }
        write_tx.commit().unwrap();

        let read_tx = db.begin_read().unwrap();
        let reader = SyncStatsReader::new(&read_tx).unwrap();
        let groups = reader.grouped_by_peer(10, 2).unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].peer_public_key, [2; 32]);
        assert_eq!(groups[1].peer_public_key, [1; 32]);
        assert_eq!(groups[1].recent_sessions.len(), 2);
        assert_eq!(groups[1].recent_sessions[0].finished_at_ms, 200);
        assert_eq!(groups[1].recent_sessions[1].finished_at_ms, 100);
    }
}
