use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::{
    crypto::AuthenticatedPeer,
    models::{
        note::{Note, NoteRecord},
        sync_stats::{SyncSessionRole, SyncSessionStatus, SyncStatsRecord, SyncStatsWriter},
    },
    service::SynapService,
};

use super::frame::FrameCodec;
use super::protocol::{
    SyncBucketEntry, SyncBucketSummary, SyncChannel, SyncConfig, SyncError, SyncMessage,
    SyncRecordId, SyncStats, PROTOCOL_VERSION,
};

const SYNC_BUCKET_COUNT: usize = (u8::MAX as usize) + 1;

fn record_bucket(record_id: SyncRecordId) -> u8 {
    record_id.0.as_bytes()[0]
}

fn bucket_digest(record_ids: &[SyncRecordId]) -> Result<Uuid, SyncError> {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_OID, b"synap.sync.bucket");
    let payload = postcard::to_allocvec(record_ids)?;
    Ok(Uuid::new_v5(&namespace, &payload))
}

fn validate_manifest(buckets: &[SyncBucketSummary]) -> Result<(), SyncError> {
    if buckets.len() != SYNC_BUCKET_COUNT {
        return Err(SyncError::InvalidManifest(format!(
            "expected {} buckets, got {}",
            SYNC_BUCKET_COUNT,
            buckets.len()
        )));
    }

    for (expected_bucket, summary) in buckets.iter().enumerate() {
        if summary.bucket != expected_bucket as u8 {
            return Err(SyncError::InvalidManifest(format!(
                "expected bucket {} at position {}, got {}",
                expected_bucket, expected_bucket, summary.bucket
            )));
        }
    }

    Ok(())
}

struct LocalArchive {
    records_by_id: BTreeMap<SyncRecordId, NoteRecord>,
    bucket_record_ids: Vec<Vec<SyncRecordId>>,
    manifest: Vec<SyncBucketSummary>,
}

impl LocalArchive {
    fn new(records: Vec<NoteRecord>) -> Result<Self, SyncError> {
        let mut records_by_id = BTreeMap::new();

        for record in records {
            let record_id = SyncRecordId::for_record(&record)?;
            match records_by_id.entry(record_id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(record);
                }
                std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &record => {
                    return Err(SyncError::RecordIdCollision { record_id });
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
        }

        let mut bucket_record_ids = vec![Vec::new(); SYNC_BUCKET_COUNT];
        for record_id in records_by_id.keys().copied() {
            bucket_record_ids[record_bucket(record_id) as usize].push(record_id);
        }

        let manifest = bucket_record_ids
            .iter()
            .enumerate()
            .map(|(bucket, record_ids)| {
                Ok(SyncBucketSummary {
                    bucket: bucket as u8,
                    record_count: record_ids.len(),
                    digest: bucket_digest(record_ids)?,
                })
            })
            .collect::<Result<Vec<_>, SyncError>>()?;

        Ok(Self {
            records_by_id,
            bucket_record_ids,
            manifest,
        })
    }

    fn manifest(&self) -> Vec<SyncBucketSummary> {
        self.manifest.clone()
    }

    fn mismatched_buckets(&self, remote_manifest: &[SyncBucketSummary]) -> BTreeSet<u8> {
        self.manifest
            .iter()
            .zip(remote_manifest)
            .filter_map(|(local, remote)| (local != remote).then_some(local.bucket))
            .collect()
    }

    fn bucket_entries(&self, buckets: &BTreeSet<u8>) -> Vec<SyncBucketEntry> {
        buckets
            .iter()
            .flat_map(|bucket| {
                self.bucket_record_ids[*bucket as usize]
                    .iter()
                    .copied()
                    .map(|record_id| SyncBucketEntry {
                        bucket: *bucket,
                        record_id,
                    })
            })
            .collect()
    }

    fn records_for(&self, record_ids: &[SyncRecordId]) -> Vec<NoteRecord> {
        let mut seen = BTreeSet::new();

        record_ids
            .iter()
            .filter(|record_id| seen.insert(**record_id))
            .filter_map(|record_id| self.records_by_id.get(record_id).cloned())
            .collect()
    }

    fn diff_against(
        &self,
        remote_inventory: &RemoteBucketInventory,
        mismatched_buckets: &BTreeSet<u8>,
    ) -> (Vec<SyncRecordId>, Vec<SyncRecordId>) {
        let mut need_from_remote = Vec::new();
        let mut need_from_local = Vec::new();

        for bucket in mismatched_buckets {
            let local_ids = &self.bucket_record_ids[*bucket as usize];
            let remote_ids = remote_inventory.ids_in_bucket(*bucket);

            for remote_id in remote_ids {
                if !self.records_by_id.contains_key(remote_id) {
                    need_from_remote.push(*remote_id);
                }
            }

            for local_id in local_ids {
                if !remote_ids.contains(local_id) {
                    need_from_local.push(*local_id);
                }
            }
        }

        (need_from_remote, need_from_local)
    }
}

struct RemoteBucketInventory {
    bucket_record_ids: Vec<BTreeSet<SyncRecordId>>,
}

impl RemoteBucketInventory {
    fn new() -> Self {
        Self {
            bucket_record_ids: vec![BTreeSet::new(); SYNC_BUCKET_COUNT],
        }
    }

    fn insert(&mut self, entry: SyncBucketEntry) -> Result<(), SyncError> {
        let expected_bucket = record_bucket(entry.record_id);
        if expected_bucket != entry.bucket {
            return Err(SyncError::BucketEntryMismatch {
                bucket: entry.bucket,
                record_id: entry.record_id,
            });
        }

        self.bucket_record_ids[entry.bucket as usize].insert(entry.record_id);
        Ok(())
    }

    fn ids_in_bucket(&self, bucket: u8) -> &BTreeSet<SyncRecordId> {
        &self.bucket_record_ids[bucket as usize]
    }

    fn validate_against(
        &self,
        expected_summaries: &BTreeMap<u8, SyncBucketSummary>,
    ) -> Result<(), SyncError> {
        for (bucket, summary) in expected_summaries {
            let received = &self.bucket_record_ids[*bucket as usize];
            if received.len() != summary.record_count {
                return Err(SyncError::BucketInventoryCountMismatch {
                    bucket: *bucket,
                    expected: summary.record_count,
                    received: received.len(),
                });
            }

            if bucket_digest(&received.iter().copied().collect::<Vec<_>>())? != summary.digest {
                return Err(SyncError::BucketInventoryDigestMismatch { bucket: *bucket });
            }
        }

        Ok(())
    }
}

struct ReceivedRecord {
    id: SyncRecordId,
    record: NoteRecord,
}

/// Transport-agnostic synchronization service for append-only Synap ledgers.
pub struct SyncService<'a> {
    core: &'a SynapService,
    config: SyncConfig,
    peer_identity: Option<SyncPeerIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPeerIdentity {
    pub key_id: Uuid,
    pub public_key: [u8; 32],
    pub fingerprint: [u8; 32],
    pub label: Option<String>,
}

impl SyncPeerIdentity {
    pub fn from_authenticated_peer(peer: &AuthenticatedPeer) -> Self {
        let identity = peer.identity();
        Self {
            key_id: identity.key_id,
            public_key: identity.public_key,
            fingerprint: identity.fingerprint,
            label: identity.label,
        }
    }
}

impl<'a> SyncService<'a> {
    pub fn new(core: &'a SynapService, config: SyncConfig) -> Self {
        Self {
            core,
            config,
            peer_identity: None,
        }
    }

    pub fn with_peer_identity(mut self, peer_identity: SyncPeerIdentity) -> Self {
        self.peer_identity = Some(peer_identity);
        self
    }

    pub fn sync_as_initiator<C: SyncChannel>(
        &self,
        channel: &mut C,
    ) -> Result<SyncStats, SyncError> {
        let mut stats = SyncStats::default();
        let started_at_ms = now_ms();
        let result = self.sync_as_initiator_inner(channel, &mut stats);
        self.finish_sync_session(SyncSessionRole::Initiator, started_at_ms, stats, result)
    }

    pub fn sync_as_responder<C: SyncChannel>(
        &self,
        channel: &mut C,
    ) -> Result<SyncStats, SyncError> {
        let mut stats = SyncStats::default();
        let started_at_ms = now_ms();
        let result = self.sync_as_responder_inner(channel, &mut stats);
        self.finish_sync_session(SyncSessionRole::Listener, started_at_ms, stats, result)
    }

    fn sync_as_initiator_inner<C: SyncChannel>(
        &self,
        channel: &mut C,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        let local = self.collect_local_archive()?;

        self.send_hello(channel, stats)?;
        self.receive_hello(channel, stats)?;

        self.send_manifest(channel, &local, stats)?;
        let remote_manifest = self.receive_manifest(channel, stats)?;

        let mismatched_buckets = local.mismatched_buckets(&remote_manifest);
        let expected_remote_buckets =
            manifest_map_for_buckets(&remote_manifest, &mismatched_buckets);
        self.send_bucket_entries(channel, &local, &mismatched_buckets, stats)?;
        let remote_inventory =
            self.receive_bucket_entries(channel, &expected_remote_buckets, stats)?;

        let (need_from_remote, need_from_local) =
            local.diff_against(&remote_inventory, &mismatched_buckets);
        let expected_from_remote: BTreeSet<_> = need_from_remote.iter().copied().collect();
        let outgoing = local.records_for(&need_from_local);

        self.send_records(channel, &outgoing, stats)?;
        let incoming = self.receive_records(channel, &expected_from_remote, stats)?;

        let (applied, skipped) = self.apply_remote_records(incoming)?;
        stats.records_applied += applied;
        stats.records_skipped += skipped;

        stats.bytes_sent += FrameCodec::write(channel, &SyncMessage::Done)?;
        let (done, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
        stats.bytes_received += bytes;
        match done {
            SyncMessage::Done => Ok(()),
            other => Err(SyncError::UnexpectedMessage {
                expected: "Done",
                got: format!("{other:?}"),
            }),
        }
    }

    fn sync_as_responder_inner<C: SyncChannel>(
        &self,
        channel: &mut C,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        let local = self.collect_local_archive()?;

        self.receive_hello(channel, stats)?;
        self.send_hello(channel, stats)?;

        let remote_manifest = self.receive_manifest(channel, stats)?;
        self.send_manifest(channel, &local, stats)?;

        let mismatched_buckets = local.mismatched_buckets(&remote_manifest);
        let expected_remote_buckets =
            manifest_map_for_buckets(&remote_manifest, &mismatched_buckets);
        let remote_inventory =
            self.receive_bucket_entries(channel, &expected_remote_buckets, stats)?;
        self.send_bucket_entries(channel, &local, &mismatched_buckets, stats)?;

        let (need_from_remote, need_from_local) =
            local.diff_against(&remote_inventory, &mismatched_buckets);
        let expected_from_remote: BTreeSet<_> = need_from_remote.iter().copied().collect();
        let outgoing = local.records_for(&need_from_local);

        let incoming = self.receive_records(channel, &expected_from_remote, stats)?;
        self.send_records(channel, &outgoing, stats)?;

        let (applied, skipped) = self.apply_remote_records(incoming)?;
        stats.records_applied += applied;
        stats.records_skipped += skipped;

        let (done, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
        stats.bytes_received += bytes;
        match done {
            SyncMessage::Done => {}
            other => {
                return Err(SyncError::UnexpectedMessage {
                    expected: "Done",
                    got: format!("{other:?}"),
                });
            }
        }
        stats.bytes_sent += FrameCodec::write(channel, &SyncMessage::Done)?;
        Ok(())
    }

    fn collect_local_archive(&self) -> Result<LocalArchive, SyncError> {
        let records = self.core.with_read(|_tx, reader| {
            let note_ids = reader
                .note_by_time()
                .map_err(redb::Error::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;

            reader.export_records(&note_ids).map_err(Into::into)
        })?;

        LocalArchive::new(records)
    }

    fn apply_remote_records(
        &self,
        records: Vec<ReceivedRecord>,
    ) -> Result<(usize, usize), SyncError> {
        let mut unique_ids = BTreeSet::new();
        let mut unique_records = Vec::new();
        let mut skipped = 0;

        for received in records {
            if unique_ids.insert(received.id) {
                unique_records.push(received.record);
            } else {
                skipped += 1;
            }
        }

        if unique_records.is_empty() {
            return Ok((0, skipped));
        }

        let applied = self
            .core
            .with_write(|tx| Note::import_records(tx, unique_records).map_err(Into::into))?;

        if applied > 0 {
            self.core.refresh_search_indexes()?;
        }

        Ok((applied, skipped))
    }

    fn send_hello<C: SyncChannel>(
        &self,
        channel: &mut C,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        stats.bytes_sent += FrameCodec::write(
            channel,
            &SyncMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )?;
        Ok(())
    }

    fn receive_hello<C: SyncChannel>(
        &self,
        channel: &mut C,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        let (message, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
        stats.bytes_received += bytes;

        match message {
            SyncMessage::Hello { version } if version == PROTOCOL_VERSION => Ok(()),
            SyncMessage::Hello { version } => Err(SyncError::ProtocolVersionMismatch {
                local: PROTOCOL_VERSION,
                remote: version,
            }),
            other => Err(SyncError::UnexpectedMessage {
                expected: "Hello",
                got: format!("{other:?}"),
            }),
        }
    }

    fn send_manifest<C: SyncChannel>(
        &self,
        channel: &mut C,
        local: &LocalArchive,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        stats.bytes_sent += FrameCodec::write(
            channel,
            &SyncMessage::Manifest {
                buckets: local.manifest(),
            },
        )?;
        Ok(())
    }

    fn receive_manifest<C: SyncChannel>(
        &self,
        channel: &mut C,
        stats: &mut SyncStats,
    ) -> Result<Vec<SyncBucketSummary>, SyncError> {
        let (message, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
        stats.bytes_received += bytes;

        match message {
            SyncMessage::Manifest { buckets } => {
                validate_manifest(&buckets)?;
                Ok(buckets)
            }
            other => Err(SyncError::UnexpectedMessage {
                expected: "Manifest",
                got: format!("{other:?}"),
            }),
        }
    }

    fn send_bucket_entries<C: SyncChannel>(
        &self,
        channel: &mut C,
        local: &LocalArchive,
        buckets: &BTreeSet<u8>,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        let entries = local.bucket_entries(buckets);

        for batch in entries.chunks(self.config.max_records_per_message.max(1)) {
            stats.bytes_sent += FrameCodec::write(
                channel,
                &SyncMessage::BucketEntries {
                    entries: batch.to_vec(),
                },
            )?;
        }

        stats.bytes_sent += FrameCodec::write(channel, &SyncMessage::BucketEntriesDone)?;
        Ok(())
    }

    fn receive_bucket_entries<C: SyncChannel>(
        &self,
        channel: &mut C,
        expected_buckets: &BTreeMap<u8, SyncBucketSummary>,
        stats: &mut SyncStats,
    ) -> Result<RemoteBucketInventory, SyncError> {
        let mut inventory = RemoteBucketInventory::new();

        loop {
            let (message, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
            stats.bytes_received += bytes;

            match message {
                SyncMessage::BucketEntries { entries } => {
                    for entry in entries {
                        if !expected_buckets.contains_key(&entry.bucket) {
                            return Err(SyncError::UnexpectedBucket {
                                bucket: entry.bucket,
                            });
                        }
                        inventory.insert(entry)?;
                    }
                }
                SyncMessage::BucketEntriesDone => break,
                other => {
                    return Err(SyncError::UnexpectedMessage {
                        expected: "BucketEntries or BucketEntriesDone",
                        got: format!("{other:?}"),
                    });
                }
            }
        }

        inventory.validate_against(expected_buckets)?;
        Ok(inventory)
    }

    fn send_records<C: SyncChannel>(
        &self,
        channel: &mut C,
        records: &[NoteRecord],
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        for batch in records.chunks(self.config.max_records_per_message.max(1)) {
            stats.records_sent += batch.len();
            stats.bytes_sent += FrameCodec::write(
                channel,
                &SyncMessage::Records {
                    records: batch.to_vec(),
                },
            )?;
        }

        stats.bytes_sent += FrameCodec::write(channel, &SyncMessage::RecordsDone)?;
        Ok(())
    }

    fn receive_records<C: SyncChannel>(
        &self,
        channel: &mut C,
        expected_record_ids: &BTreeSet<SyncRecordId>,
        stats: &mut SyncStats,
    ) -> Result<Vec<ReceivedRecord>, SyncError> {
        let mut records = Vec::new();
        let mut received_ids = BTreeSet::new();

        loop {
            let (message, bytes) = FrameCodec::read::<SyncMessage>(channel)?;
            stats.bytes_received += bytes;

            match message {
                SyncMessage::Records { records: batch } => {
                    stats.records_received += batch.len();

                    for record in batch {
                        let record_id = SyncRecordId::for_record(&record)?;
                        if !expected_record_ids.contains(&record_id) {
                            return Err(SyncError::UnexpectedRecord { record_id });
                        }

                        received_ids.insert(record_id);
                        records.push(ReceivedRecord {
                            id: record_id,
                            record,
                        });
                    }
                }
                SyncMessage::RecordsDone => break,
                other => {
                    return Err(SyncError::UnexpectedMessage {
                        expected: "Records or RecordsDone",
                        got: format!("{other:?}"),
                    });
                }
            }
        }

        if received_ids.len() != expected_record_ids.len() {
            return Err(SyncError::IncompleteRecordTransfer {
                expected: expected_record_ids.len(),
                received: received_ids.len(),
            });
        }

        Ok(records)
    }

    fn finish_sync_session(
        &self,
        role: SyncSessionRole,
        started_at_ms: u64,
        mut stats: SyncStats,
        result: Result<(), SyncError>,
    ) -> Result<SyncStats, SyncError> {
        let finished_at_ms = now_ms();
        stats.duration_ms = finished_at_ms.saturating_sub(started_at_ms);

        let status = if result.is_ok() {
            SyncSessionStatus::Completed
        } else {
            SyncSessionStatus::Failed
        };
        let error_message = result.as_ref().err().map(ToString::to_string);

        if let Err(record_err) = self.record_sync_session(
            role,
            status,
            started_at_ms,
            finished_at_ms,
            &stats,
            error_message,
        ) {
            if result.is_ok() {
                return Err(record_err);
            }
        }

        result.map(|()| stats)
    }

    fn record_sync_session(
        &self,
        role: SyncSessionRole,
        status: SyncSessionStatus,
        started_at_ms: u64,
        finished_at_ms: u64,
        stats: &SyncStats,
        error_message: Option<String>,
    ) -> Result<(), SyncError> {
        let Some(peer_identity) = self.peer_identity.clone() else {
            return Ok(());
        };

        let stats_record = SyncStatsRecord {
            id: Uuid::now_v7(),
            role,
            status,
            peer_key_id: Some(peer_identity.key_id),
            peer_public_key: Some(peer_identity.public_key.to_vec()),
            peer_fingerprint: Some(peer_identity.fingerprint.to_vec()),
            peer_label: peer_identity.label,
            started_at_ms,
            finished_at_ms,
            records_sent: stats.records_sent as u64,
            records_received: stats.records_received as u64,
            records_applied: stats.records_applied as u64,
            records_skipped: stats.records_skipped as u64,
            bytes_sent: stats.bytes_sent as u64,
            bytes_received: stats.bytes_received as u64,
            duration_ms: stats.duration_ms,
            error_message,
        };

        self.core
            .with_write(|tx| {
                let writer = SyncStatsWriter::new(tx);
                writer.put(&stats_record)?;
                Ok(())
            })
            .map_err(Into::into)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn manifest_map_for_buckets(
    manifest: &[SyncBucketSummary],
    buckets: &BTreeSet<u8>,
) -> BTreeMap<u8, SyncBucketSummary> {
    manifest
        .iter()
        .filter(|summary| buckets.contains(&summary.bucket))
        .cloned()
        .map(|summary| (summary.bucket, summary))
        .collect()
}
