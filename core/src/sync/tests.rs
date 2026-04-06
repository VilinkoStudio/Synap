use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{Arc, Condvar, Mutex},
};

use tempfile::tempdir;

use crate::{
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
