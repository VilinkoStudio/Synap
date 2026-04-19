use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{Arc, Condvar, Mutex},
};

use tempfile::tempdir;

use crate::{
    models::sync_stats::{SyncSessionRole, SyncSessionStatus, SyncStatsReader, SyncStatsRecord},
    service::SynapService,
    sync::{SyncConfig, SyncService, SyncStats},
};

struct PipeState {
    buffer: Mutex<VecDeque<u8>>,
    ready: Condvar,
}

impl PipeState {
    fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }
}

struct MemoryChannel {
    inbound: Arc<PipeState>,
    outbound: Arc<PipeState>,
}

impl MemoryChannel {
    fn pair() -> (Self, Self) {
        let a_to_b = Arc::new(PipeState::new());
        let b_to_a = Arc::new(PipeState::new());

        (
            Self {
                inbound: b_to_a.clone(),
                outbound: a_to_b.clone(),
            },
            Self {
                inbound: a_to_b,
                outbound: b_to_a,
            },
        )
    }
}

impl Read for MemoryChannel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut guard = self.inbound.buffer.lock().unwrap();
        while guard.is_empty() {
            guard = self.inbound.ready.wait(guard).unwrap();
        }

        let len = buf.len().min(guard.len());
        for (dst, byte) in buf.iter_mut().zip(guard.drain(..len)) {
            *dst = byte;
        }

        Ok(len)
    }
}

impl Write for MemoryChannel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.outbound.buffer.lock().unwrap();
        guard.extend(buf.iter().copied());
        drop(guard);
        self.outbound.ready.notify_all();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_bidirectional_sync(
    service_a: &SynapService,
    service_b: &SynapService,
) -> (SyncStats, SyncStats) {
    run_bidirectional_sync_with_config(service_a, service_b, SyncConfig::default())
}

fn run_bidirectional_sync_with_config(
    service_a: &SynapService,
    service_b: &SynapService,
    config: SyncConfig,
) -> (SyncStats, SyncStats) {
    let (mut channel_a, mut channel_b) = MemoryChannel::pair();

    std::thread::scope(|scope| {
        let sync_a = SyncService::new(service_a, config.clone());
        let sync_b = SyncService::new(service_b, config);

        let initiator = scope.spawn(move || sync_a.sync_as_initiator(&mut channel_a));
        let responder = scope.spawn(move || sync_b.sync_as_responder(&mut channel_b));

        (
            initiator.join().unwrap().unwrap(),
            responder.join().unwrap().unwrap(),
        )
    })
}

fn run_facade_sync(
    service_a: &SynapService,
    service_b: &SynapService,
) -> (
    Result<crate::dto::SyncSessionDTO, crate::error::ServiceError>,
    Result<crate::dto::SyncSessionDTO, crate::error::ServiceError>,
) {
    let (channel_a, channel_b) = MemoryChannel::pair();

    std::thread::scope(|scope| {
        let initiator = scope.spawn(move || service_a.initiate_sync(channel_a));
        let listener = scope.spawn(move || service_b.listen_sync(channel_b));

        (initiator.join().unwrap(), listener.join().unwrap())
    })
}

fn read_sync_records(service: &SynapService) -> Vec<SyncStatsRecord> {
    service
        .with_read(|tx, _reader| {
            let reader = SyncStatsReader::new(tx)?;
            let records = reader
                .all()
                .map_err(redb::Error::from)?
                .map(|item| item.map_err(redb::Error::from))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(records)
        })
        .unwrap()
}

#[test]
fn test_bidirectional_sync_merges_append_only_ledgers() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("peer-a.redb");
    let path_b = dir.path().join("peer-b.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let root_a = service_a
        .create_note("learn rust".to_string(), vec!["rust".into()])
        .unwrap();
    let reply_a = service_a
        .reply_note(&root_a.id, "child reply".to_string(), vec!["thread".into()])
        .unwrap();
    let edited_a = service_a
        .edit_note(
            &root_a.id,
            "learn rust async".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();
    service_a.delete_note(&reply_a.id).unwrap();

    let note_b = service_b
        .create_note("learn python".to_string(), vec!["python".into()])
        .unwrap();

    run_bidirectional_sync(&service_a, &service_b);

    let rust_hits_b = service_b.search("rust async", 10).unwrap();
    assert!(rust_hits_b.iter().any(|note| note.id == edited_a.id));

    let python_hits_a = service_a.search("python", 10).unwrap();
    assert!(python_hits_a.iter().any(|note| note.id == note_b.id));

    let tag_hits_a = service_a.search_tags("async", 10).unwrap();
    assert!(tag_hits_a.iter().any(|tag| tag == "async"));

    let tag_hits_b = service_b.search_tags("python", 10).unwrap();
    assert!(tag_hits_b.iter().any(|tag| tag == "python"));

    let replies_b = service_b.get_replies(&root_a.id, None, 10).unwrap();
    assert!(replies_b.is_empty());

    let deleted_seen = service_b
        .with_read(|_tx, reader| {
            Ok(reader
                .deleted_note_ids()
                .map_err(redb::Error::from)?
                .map(|item| item.map_err(redb::Error::from))
                .collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap();
    assert!(deleted_seen.iter().any(|id| id.to_string() == reply_a.id));
}

#[test]
fn test_bidirectional_sync_skips_records_when_peers_are_already_aligned() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("peer-a-aligned.redb");
    let path_b = dir.path().join("peer-b-aligned.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let root = service_a
        .create_note("learn rust".to_string(), vec!["rust".into()])
        .unwrap();
    service_a
        .edit_note(
            &root.id,
            "learn rust async".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();

    let first_stats = run_bidirectional_sync(&service_a, &service_b);
    assert!(first_stats.0.records_sent > 0 || first_stats.1.records_sent > 0);

    let second_stats = run_bidirectional_sync(&service_a, &service_b);
    assert_eq!(second_stats.0.records_sent, 0);
    assert_eq!(second_stats.0.records_received, 0);
    assert_eq!(second_stats.0.records_applied, 0);
    assert_eq!(second_stats.1.records_sent, 0);
    assert_eq!(second_stats.1.records_received, 0);
    assert_eq!(second_stats.1.records_applied, 0);
}

#[test]
fn test_bidirectional_sync_handles_chunked_bucket_entries() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("peer-a-chunked.redb");
    let path_b = dir.path().join("peer-b-chunked.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let mut created_ids = Vec::new();
    for idx in 0..8 {
        let note = service_a
            .create_note(format!("chunked note {}", idx), vec!["chunk".into()])
            .unwrap();
        created_ids.push(note.id);
    }

    run_bidirectional_sync_with_config(
        &service_a,
        &service_b,
        SyncConfig {
            max_records_per_message: 1,
        },
    );

    for note_id in created_ids {
        let note = service_b.get_note(&note_id).unwrap();
        assert_eq!(note.id, note_id);
    }
}

#[test]
fn test_sync_facade_runs_through_crypto_channel() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("peer-a-facade.redb");
    let path_b = dir.path().join("peer-b-facade.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let local_a = service_a.get_local_identity().unwrap();
    let local_b = service_b.get_local_identity().unwrap();
    service_a
        .trust_peer(&local_b.signing.public_key, Some("peer-b".into()))
        .unwrap();
    service_b
        .trust_peer(&local_a.signing.public_key, Some("peer-a".into()))
        .unwrap();

    let note_a = service_a
        .create_note("facade sync note".to_string(), vec!["facade".into()])
        .unwrap();

    let (initiator, listener) = run_facade_sync(&service_a, &service_b);
    let initiator = initiator.unwrap();
    let listener = listener.unwrap();

    assert_eq!(initiator.peer.public_key, local_b.signing.public_key);
    assert_eq!(listener.peer.public_key, local_a.signing.public_key);
    assert!(
        initiator
            .stats
            .as_ref()
            .is_some_and(|stats| stats.records_sent > 0)
            || listener
                .stats
                .as_ref()
                .is_some_and(|stats| stats.records_sent > 0)
    );

    let fetched = service_b.get_note(&note_a.id).unwrap();
    assert_eq!(fetched.content, "facade sync note");

    let records_a = read_sync_records(&service_a);
    let records_b = read_sync_records(&service_b);
    assert_eq!(records_a.len(), 1);
    assert_eq!(records_b.len(), 1);
    assert_eq!(records_a[0].role, SyncSessionRole::Initiator);
    assert_eq!(records_b[0].role, SyncSessionRole::Listener);
    assert_eq!(records_a[0].status, SyncSessionStatus::Completed);
    assert_eq!(records_b[0].status, SyncSessionStatus::Completed);
    assert_eq!(
        records_a[0].peer_public_key.as_deref(),
        Some(local_b.signing.public_key.as_slice())
    );
    assert_eq!(
        records_b[0].peer_public_key.as_deref(),
        Some(local_a.signing.public_key.as_slice())
    );
    assert_eq!(records_a[0].peer_label.as_deref(), Some("peer-b"));
    assert_eq!(records_b[0].peer_label.as_deref(), Some("peer-a"));
}

#[test]
fn test_sync_facade_remembers_untrusted_peers_until_approved() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("peer-a-pending.redb");
    let path_b = dir.path().join("peer-b-pending.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let local_a = service_a.get_local_identity().unwrap();
    let local_b = service_b.get_local_identity().unwrap();

    let (initiator, listener) = run_facade_sync(&service_a, &service_b);

    let initiator = initiator.unwrap();
    let listener = listener.unwrap();
    assert_eq!(initiator.status, crate::dto::SyncStatusDTO::PendingTrust);
    assert_eq!(listener.status, crate::dto::SyncStatusDTO::PendingTrust);
    assert_eq!(initiator.peer.public_key, local_b.signing.public_key);
    assert_eq!(listener.peer.public_key, local_a.signing.public_key);

    let pending_a = service_a.get_peers().unwrap();
    let pending_b = service_b.get_peers().unwrap();
    assert!(pending_a
        .iter()
        .any(|record| record.public_key == local_b.signing.public_key));
    assert!(pending_b
        .iter()
        .any(|record| record.public_key == local_a.signing.public_key));

    service_a
        .trust_peer(&local_b.signing.public_key, Some("peer-b".into()))
        .unwrap();
    service_b
        .trust_peer(&local_a.signing.public_key, Some("peer-a".into()))
        .unwrap();

    let note_b = service_b
        .create_note(
            "trusted after approval".to_string(),
            vec!["approval".into()],
        )
        .unwrap();

    let (initiator, listener) = run_facade_sync(&service_a, &service_b);
    assert!(initiator.is_ok());
    assert!(listener.is_ok());

    let fetched = service_a.get_note(&note_b.id).unwrap();
    assert_eq!(fetched.content, "trusted after approval");
}
