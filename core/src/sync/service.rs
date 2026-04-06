use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use uuid::Uuid;

use crate::{
    models::note::{Note, NoteRecord},
    service::SynapService,
};

use super::protocol::{
    FrameCodec, SyncBucketEntry, SyncBucketSummary, SyncChannel, SyncConfig, SyncError,
    SyncMessage, SyncRecordId, SyncStats, PROTOCOL_VERSION,
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
                std::collections::btree_map::Entry::Occupied(entry)
                    if entry.get() != &record =>
                {
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
}

struct ReceivedRecord {
    id: SyncRecordId,
    record: NoteRecord,
}

/// Transport-agnostic synchronization service for append-only Synap ledgers.
pub struct SyncService<'a> {
    core: &'a SynapService,
    config: SyncConfig,
}

impl<'a> SyncService<'a> {
    pub fn new(core: &'a SynapService, config: SyncConfig) -> Self {
        Self { core, config }
    }

    pub fn sync_as_initiator<C: SyncChannel>(
        &self,
        channel: &mut C,
    ) -> Result<SyncStats, SyncError> {
        let started = Instant::now();
        let mut stats = SyncStats::default();
        let local = self.collect_local_archive()?;

        self.send_hello(channel, &mut stats)?;
        self.receive_hello(channel, &mut stats)?;

        self.send_manifest(channel, &local, &mut stats)?;
        let remote_manifest = self.receive_manifest(channel, &mut stats)?;

        let mismatched_buckets = local.mismatched_buckets(&remote_manifest);
        self.send_bucket_entries(channel, &local, &mismatched_buckets, &mut stats)?;
        let remote_inventory =
            self.receive_bucket_entries(channel, &mismatched_buckets, &mut stats)?;

        let (need_from_remote, need_from_local) =
            local.diff_against(&remote_inventory, &mismatched_buckets);
        let expected_from_remote: BTreeSet<_> = need_from_remote.iter().copied().collect();
        let outgoing = local.records_for(&need_from_local);

        self.send_records(channel, &outgoing, &mut stats)?;
        let incoming = self.receive_records(channel, &expected_from_remote, &mut stats)?;

        let (applied, skipped) = self.apply_remote_records(incoming)?;
        stats.records_applied += applied;
        stats.records_skipped += skipped;

        stats.bytes_sent += FrameCodec::write_message(channel, &SyncMessage::Done)?;
        let (done, bytes) = FrameCodec::read_message(channel)?;
        stats.bytes_received += bytes;
        match done {
            SyncMessage::Done => {}
            other => {
                return Err(SyncError::UnexpectedMessage {
                    expected: "Done",
                    got: other,
                });
            }
        }

        channel.close()?;
        stats.duration_ms = started.elapsed().as_millis() as u64;
        Ok(stats)
    }

    pub fn sync_as_responder<C: SyncChannel>(
        &self,
        channel: &mut C,
    ) -> Result<SyncStats, SyncError> {
        let started = Instant::now();
        let mut stats = SyncStats::default();
        let local = self.collect_local_archive()?;

        self.receive_hello(channel, &mut stats)?;
        self.send_hello(channel, &mut stats)?;

        let remote_manifest = self.receive_manifest(channel, &mut stats)?;
        self.send_manifest(channel, &local, &mut stats)?;

        let mismatched_buckets = local.mismatched_buckets(&remote_manifest);
        let remote_inventory =
            self.receive_bucket_entries(channel, &mismatched_buckets, &mut stats)?;
        self.send_bucket_entries(channel, &local, &mismatched_buckets, &mut stats)?;

        let (need_from_remote, need_from_local) =
            local.diff_against(&remote_inventory, &mismatched_buckets);
        let expected_from_remote: BTreeSet<_> = need_from_remote.iter().copied().collect();
        let outgoing = local.records_for(&need_from_local);

        let incoming = self.receive_records(channel, &expected_from_remote, &mut stats)?;
        self.send_records(channel, &outgoing, &mut stats)?;

        let (applied, skipped) = self.apply_remote_records(incoming)?;
        stats.records_applied += applied;
        stats.records_skipped += skipped;

        let (done, bytes) = FrameCodec::read_message(channel)?;
        stats.bytes_received += bytes;
        match done {
            SyncMessage::Done => {}
            other => {
                return Err(SyncError::UnexpectedMessage {
                    expected: "Done",
                    got: other,
                });
            }
        }
        stats.bytes_sent += FrameCodec::write_message(channel, &SyncMessage::Done)?;

        channel.close()?;
        stats.duration_ms = started.elapsed().as_millis() as u64;
        Ok(stats)
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
        stats.bytes_sent += FrameCodec::write_message(
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
        let (message, bytes) = FrameCodec::read_message(channel)?;
        stats.bytes_received += bytes;

        match message {
            SyncMessage::Hello { version } if version == PROTOCOL_VERSION => Ok(()),
            SyncMessage::Hello { version } => Err(SyncError::ProtocolVersionMismatch {
                local: PROTOCOL_VERSION,
                remote: version,
            }),
            other => Err(SyncError::UnexpectedMessage {
                expected: "Hello",
                got: other,
            }),
        }
    }

    fn send_manifest<C: SyncChannel>(
        &self,
        channel: &mut C,
        local: &LocalArchive,
        stats: &mut SyncStats,
    ) -> Result<(), SyncError> {
        stats.bytes_sent += FrameCodec::write_message(
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
        let (message, bytes) = FrameCodec::read_message(channel)?;
        stats.bytes_received += bytes;

        match message {
            SyncMessage::Manifest { buckets } => {
                validate_manifest(&buckets)?;
                Ok(buckets)
            }
            other => Err(SyncError::UnexpectedMessage {
                expected: "Manifest",
                got: other,
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
            stats.bytes_sent += FrameCodec::write_message(
                channel,
                &SyncMessage::BucketEntries {
                    entries: batch.to_vec(),
                },
            )?;
        }

        stats.bytes_sent += FrameCodec::write_message(channel, &SyncMessage::BucketEntriesDone)?;
        Ok(())
    }

    fn receive_bucket_entries<C: SyncChannel>(
        &self,
        channel: &mut C,
        expected_buckets: &BTreeSet<u8>,
        stats: &mut SyncStats,
    ) -> Result<RemoteBucketInventory, SyncError> {
        let mut inventory = RemoteBucketInventory::new();

        loop {
            let (message, bytes) = FrameCodec::read_message(channel)?;
            stats.bytes_received += bytes;

            match message {
                SyncMessage::BucketEntries { entries } => {
                    for entry in entries {
                        if !expected_buckets.contains(&entry.bucket) {
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
                        got: other,
                    });
                }
            }
        }

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
            stats.bytes_sent += FrameCodec::write_message(
                channel,
                &SyncMessage::Records {
                    records: batch.to_vec(),
                },
            )?;
        }

        stats.bytes_sent += FrameCodec::write_message(channel, &SyncMessage::RecordsDone)?;
        Ok(())
    }

    fn receive_records<C: SyncChannel>(
        &self,
        channel: &mut C,
        expected_record_ids: &BTreeSet<SyncRecordId>,
        stats: &mut SyncStats,
    ) -> Result<Vec<ReceivedRecord>, SyncError> {
        let mut records = Vec::new();

        loop {
            let (message, bytes) = FrameCodec::read_message(channel)?;
            stats.bytes_received += bytes;

            match message {
                SyncMessage::Records { records: batch } => {
                    stats.records_received += batch.len();

                    for record in batch {
                        let record_id = SyncRecordId::for_record(&record)?;
                        if !expected_record_ids.contains(&record_id) {
                            return Err(SyncError::UnexpectedRecord { record_id });
                        }

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
                        got: other,
                    });
                }
            }
        }

        Ok(records)
    }
}
