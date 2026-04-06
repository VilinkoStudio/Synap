use std::time::Instant;

use uuid::Uuid;

use crate::{
    error::ServiceError,
    models::note::{Note, NoteRecord},
    service::SynapService,
    sync::protocol::SyncChannel,
    sync::share::{FrameCodec, ShareHeader, ShareMessage, ShareStats},
};

const SHARE_BATCH_SIZE: usize = 256;

pub struct ShareService<'a> {
    core: &'a SynapService,
}

impl<'a> ShareService<'a> {
    pub fn new(core: &'a SynapService) -> Self {
        Self { core }
    }

    pub fn collect_selected_records(
        &self,
        note_ids: &[Uuid],
    ) -> Result<Vec<NoteRecord>, ServiceError> {
        self.core
            .with_read(|_tx, reader| reader.export_records(note_ids).map_err(Into::into))
    }

    pub fn send<C: SyncChannel>(
        &self,
        channel: &mut C,
        records: Vec<NoteRecord>,
    ) -> Result<ShareStats, ServiceError> {
        let started = Instant::now();
        let mut stats = ShareStats::default();

        let header = ShareHeader {
            version: crate::sync::share::SHARE_VERSION,
            record_count: records.len(),
            total_size_hint: 0,
        };
        stats.bytes_sent += FrameCodec::write_message(channel, &ShareMessage::Header(header))?;

        for batch in records.chunks(SHARE_BATCH_SIZE) {
            stats.records_sent += batch.len();
            stats.bytes_sent += FrameCodec::write_message(
                channel,
                &ShareMessage::Records {
                    records: batch.to_vec(),
                },
            )?;
        }

        let final_stats = ShareStats {
            records_sent: stats.records_sent,
            bytes_sent: stats.bytes_sent,
            duration_ms: 0,
        };
        stats.bytes_sent +=
            FrameCodec::write_message(channel, &ShareMessage::Done { stats: final_stats })?;
        stats.duration_ms = started.elapsed().as_millis() as u64;

        Ok(stats)
    }

    pub fn receive<C: SyncChannel>(&self, channel: &mut C) -> Result<ShareStats, ServiceError> {
        let started = Instant::now();
        let mut stats = ShareStats::default();

        let header = match FrameCodec::read_message(channel) {
            Ok(ShareMessage::Header(header)) => header,
            Ok(other) => {
                return Err(ServiceError::ShareProtocol(format!(
                    "expected Header, got {:?}",
                    other
                )));
            }
            Err(err) => {
                return Err(ServiceError::ShareProtocol(format!(
                    "failed to read header: {}",
                    err
                )));
            }
        };

        let mut all_records = Vec::with_capacity(header.record_count);
        let mut done = false;

        while !done {
            match FrameCodec::read_message(channel) {
                Ok(ShareMessage::Records { records }) => {
                    all_records.extend(records);
                }
                Ok(ShareMessage::Done { stats: peer_stats }) => {
                    stats.records_sent = peer_stats.records_sent;
                    stats.bytes_sent = peer_stats.bytes_sent;
                    done = true;
                }
                Ok(other) => {
                    return Err(ServiceError::ShareProtocol(format!(
                        "unexpected message: {:?}",
                        other
                    )));
                }
                Err(err) => {
                    return Err(ServiceError::ShareProtocol(format!(
                        "failed to read message: {}",
                        err
                    )));
                }
            }
        }

        self.apply_records(all_records)?;
        stats.duration_ms = started.elapsed().as_millis() as u64;

        Ok(stats)
    }

    fn apply_records(&self, records: Vec<NoteRecord>) -> Result<usize, ServiceError> {
        let applied = self
            .core
            .with_write(|tx| Note::import_records(tx, records).map_err(Into::into))?;

        if applied > 0 {
            self.core.refresh_search_indexes()?;
        }

        Ok(applied)
    }
}
