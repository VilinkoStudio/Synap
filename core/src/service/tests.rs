use super::*;
use crate::dto::DatabaseTableKindDTO;
use crate::EmbeddingConfig;
use crate::models::{
    embedding_cache::EmbeddingCacheStamp,
    tag_profile::{TagProfileMetadata, TagProfileRecord},
};
use crate::nlp::embedding::{EmbeddingError, EmbeddingModel, LocalHashEmbedding};
use std::{
    net::TcpListener,
    sync::{Arc, Condvar, Mutex as StdMutex, mpsc},
    thread,
    time::Duration,
};
use tempfile::tempdir;
use uuid::Uuid;

fn seed_db(path: &Path, tags: &[&str]) {
    let db = Database::create(path).unwrap();

    let tx = db.begin_write().unwrap();
    Note::init_schema(&tx).unwrap();
    TagWriter::init_schema(&tx).unwrap();

    let tag_writer = TagWriter::new(&tx);
    let mut materialized = Vec::new();
    for tag in tags {
        materialized.push(tag_writer.find_or_create(*tag).unwrap());
    }

    // search_tags 只返回挂有 live note 的 tag
    if !materialized.is_empty() {
        Note::create(&tx, "seed note".into(), materialized).unwrap();
    }

    tx.commit().unwrap();
}

#[test]
fn test_search_tags_uses_initialized_tag_index() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    seed_db(&db_path, &["rust", "python", "async-rust"]);

    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let results = service.search_tags("rust", 10).unwrap();
    assert!(results.iter().any(|tag| tag == "rust"));
    assert!(results.iter().any(|tag| tag == "async-rust"));
    assert!(!results.iter().any(|tag| tag == "python"));
}

#[test]
fn database_storage_metrics_reports_initialized_tables() {
    let service = SynapService::open_memory().unwrap();

    let metrics = service.database_storage_metrics().unwrap();

    assert!(metrics.page_size > 0);
    assert!(metrics.tables.iter().any(|table| table.name == "NoteBlocks"));
    assert!(metrics.tables.iter().any(|table| {
        table.name == "TagToNotes" && table.kind == DatabaseTableKindDTO::MultimapTable
    }));
}

#[test]
fn test_open_existing_db_auto_creates_crypto_schema_and_identity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    seed_db(&db_path, &["rust"]);

    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
    let local_identity = service.get_local_identity().unwrap();
    assert_eq!(local_identity.identity.public_key.len(), 32);
    assert_eq!(local_identity.signing.public_key.len(), 32);
    assert!(!local_identity.identity.avatar_png.is_empty());
    assert!(!local_identity.signing.avatar_png.is_empty());
    assert!(!local_identity.identity.kaomoji_fingerprint.is_empty());
    assert!(!local_identity.signing.kaomoji_fingerprint.is_empty());

    drop(service);

    let reopened = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
    let reopened_identity = reopened.get_local_identity().unwrap();
    assert_eq!(
        reopened_identity.identity.public_key,
        local_identity.identity.public_key
    );
    assert_eq!(
        reopened_identity.signing.public_key,
        local_identity.signing.public_key
    );
}

#[test]
fn test_relay_peer_inventory_cache_roundtrip() {
    let service = SynapService::open_memory().unwrap();
    let peer_public_key = [3u8; 32];
    let inventory = RelayInventory {
        version: RelayInventory::VERSION,
        records: vec![crate::sync::RelayRecordDescriptor {
            root_note_id: Uuid::from_u128(31),
            sync_id: crate::sync::SyncRecordId(Uuid::from_u128(32)),
        }],
    };

    let stored = service
        .cache_relay_peer_inventory(&peer_public_key, inventory.clone(), 12345)
        .unwrap();

    let fetched = service.get_relay_peer(&peer_public_key).unwrap().unwrap();
    assert_eq!(fetched, stored);
    assert_eq!(fetched.cached_inventory, inventory);

    let all = service.list_relay_peers().unwrap();
    assert_eq!(all, vec![stored]);

    service.delete_relay_peer(&peer_public_key).unwrap();
    assert!(service.get_relay_peer(&peer_public_key).unwrap().is_none());
}

#[test]
fn test_recommend_tag_returns_related_tags() {
    let service = SynapService::new(None).unwrap();

    service
        .create_note(
            "Rust ownership and lifetimes for async services".to_string(),
            vec!["rust".into(), "async".into(), "backend".into()],
        )
        .unwrap();
    service
        .create_note(
            "Tokio runtime, future polling and async scheduling".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();
    service
        .create_note(
            "数据库索引与查询优化实践".to_string(),
            vec!["database".into(), "backend".into()],
        )
        .unwrap();
    service.backfill_note_embeddings().unwrap();

    let tags = service.recommend_tag("tokio async ownership", 3).unwrap();
    assert!(tags.iter().any(|tag| tag == "rust"));
    assert!(tags.iter().any(|tag| tag == "async"));
}

#[test]
fn test_recommend_tag_tracks_note_lifecycle() {
    let service = SynapService::new(None).unwrap();

    let original = service
        .create_note("tokio future runtime".to_string(), vec!["async".into()])
        .unwrap();
    service.backfill_note_embeddings().unwrap();

    let initial = service.recommend_tag("tokio runtime", 3).unwrap();
    assert!(initial.iter().any(|tag| tag == "async"));

    let edited = service
        .edit_note(
            &original.id,
            "sql index join planner".to_string(),
            vec!["database".into()],
        )
        .unwrap();
    service.backfill_note_embeddings().unwrap();

    let updated = service.recommend_tag("sql planner", 3).unwrap();
    assert!(updated.iter().any(|tag| tag == "database"));

    let old_query = service.recommend_tag("tokio runtime", 3).unwrap();
    assert!(!old_query.iter().any(|tag| tag == "async"));

    service.delete_note(&edited.id).unwrap();
    let after_delete = service.recommend_tag("sql planner", 3).unwrap();
    assert!(!after_delete.iter().any(|tag| tag == "database"));

    service.restore_note(&edited.id).unwrap();
    service.backfill_note_embeddings().unwrap();
    let after_restore = service.recommend_tag("sql planner", 3).unwrap();
    assert!(after_restore.iter().any(|tag| tag == "database"));
}

#[test]
fn test_tag_profiles_persist_across_reopen_without_rebuild() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("tag-profiles.redb");
    let service = SynapService::open(&db_path).unwrap();
    service
        .create_note(
            "tokio async runtime scheduling".into(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();
    service.backfill_note_embeddings().unwrap();
    let generation = service
        .with_read(|tx, _| Ok(TagProfileStore::load_metadata(tx)?.unwrap().generation))
        .unwrap();
    assert!(generation > 0);
    drop(service);

    let reopened = SynapService::open(&db_path).unwrap();
    let reopened_generation = reopened
        .with_read(|tx, _| Ok(TagProfileStore::load_metadata(tx)?.unwrap().generation))
        .unwrap();
    assert_eq!(reopened_generation, generation);
    let tags = reopened.recommend_tag("tokio runtime", 2).unwrap();
    assert!(tags.iter().any(|tag| tag == "rust"));
    assert!(tags.iter().any(|tag| tag == "async"));
}

#[test]
fn test_legacy_note_vectors_are_adopted_into_tag_profiles() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("legacy-vectors.redb");
    let db = Database::create(&db_path).unwrap();
    let tx = db.begin_write().unwrap();
    Note::init_schema(&tx).unwrap();
    TagWriter::init_schema(&tx).unwrap();
    let tag = TagWriter::new(&tx).find_or_create("rust").unwrap();
    let note = Note::create(&tx, "rust ownership borrowing".into(), vec![tag]).unwrap();
    let vector = LocalHashEmbedding::new(384)
        .embed("rust ownership borrowing")
        .unwrap();
    Note::vector_index()
        .put(&tx, note.get_id().as_bytes(), &vector)
        .unwrap();
    tx.commit().unwrap();
    drop(db);

    let service = SynapService::open(&db_path).unwrap();
    let tags = service.recommend_tag("rust borrowing", 1).unwrap();
    assert_eq!(tags, ["rust"]);
    service
        .with_read(|tx, _| {
            assert!(EmbeddingCacheMetadata::load(tx)?.is_some());
            let metadata = TagProfileStore::load_metadata(tx)?.unwrap();
            assert_eq!(metadata.generation, 1);
            assert_eq!(TagProfileStore::load_profiles(tx)?.len(), 1);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_unsupported_embedding_cache_version_invalidates_derived_profiles() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("embedding-version.redb");
    let service = SynapService::open(&db_path).unwrap();
    service
        .create_note("rust ownership borrowing".into(), vec!["rust".into()])
        .unwrap();
    service.backfill_note_embeddings().unwrap();
    assert_eq!(
        service.recommend_tag("rust borrowing", 1).unwrap(),
        ["rust"]
    );
    drop(service);

    let db = Database::open(&db_path).unwrap();
    let tx = db.begin_write().unwrap();
    let mut incompatible = EmbeddingCacheStamp::new("local-hash:v1:384".into(), 384);
    incompatible.schema_version += 1;
    EmbeddingCacheMetadata::save(&tx, &incompatible).unwrap();
    tx.commit().unwrap();
    drop(db);

    let reopened = SynapService::open(&db_path).unwrap();
    assert!(
        reopened
            .recommend_tag("rust borrowing", 1)
            .unwrap()
            .is_empty()
    );
    assert_eq!(reopened.backfill_note_embeddings().unwrap(), 1);
    assert_eq!(
        reopened.recommend_tag("rust borrowing", 1).unwrap(),
        ["rust"]
    );
}

#[test]
fn test_interrupted_tag_profile_migration_resumes_from_building_state() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("interrupted-profile-migration.redb");
    let service = SynapService::open(&db_path).unwrap();
    service
        .create_note("rust ownership borrowing".into(), vec!["rust".into()])
        .unwrap();
    service.backfill_note_embeddings().unwrap();
    drop(service);

    let db = Database::open(&db_path).unwrap();
    let tx = db.begin_write().unwrap();
    let building = TagProfileMetadata::new("local-hash:v1:384".into(), 384);
    assert!(!building.is_ready());
    TagProfileStore::reset(&tx, &building).unwrap();
    tx.commit().unwrap();
    drop(db);

    let reopened = SynapService::open(&db_path).unwrap();
    assert_eq!(
        reopened.recommend_tag("rust borrowing", 1).unwrap(),
        ["rust"]
    );
    reopened
        .with_read(|tx, _| {
            assert!(TagProfileStore::load_metadata(tx)?.unwrap().is_ready());
            assert_eq!(TagProfileStore::load_profiles(tx)?.len(), 1);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_tag_recommender_rejects_older_same_space_snapshot() {
    fn index(generation: u64) -> TagProfileIndex {
        let metadata = TagProfileMetadata {
            schema_version: 1,
            algorithm_version: 1,
            embedding_space: "local-hash:test".into(),
            dimension: 2,
            generation,
        };
        TagProfileIndex::build(
            &metadata,
            vec![TagProfileRecord {
                tag_id: [1; 16],
                name: format!("generation-{generation}"),
                embedding_sum: vec![1.0, 0.0],
                embedding_count: 1,
            }],
            None,
        )
    }

    let recommender = ServiceTagRecommender::new();
    assert!(recommender.replace_if_newer(index(3)));
    assert!(!recommender.replace_if_newer(index(2)));
    let current = recommender.index.read().unwrap();
    assert_eq!(current.generation(), 3);
    assert_eq!(current.recommend_tags(&[1.0, 0.0], 1), ["generation-3"]);
}

#[test]
fn test_semantic_search_initializes_from_existing_notes() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    service
        .create_note(
            "rust async runtime ownership".to_string(),
            vec!["rust".into()],
        )
        .unwrap();
    service
        .create_note(
            "gardening watering schedule".to_string(),
            vec!["life".into()],
        )
        .unwrap();
    assert_eq!(service.backfill_note_embeddings().unwrap(), 2);

    drop(service);

    let reopened = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
    let results = reopened.search_semantic("async ownership", 5).unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].content, "rust async runtime ownership");
}

#[test]
fn test_semantic_search_tracks_note_lifecycle() {
    let service = SynapService::new(None).unwrap();

    let original = service
        .create_note("tokio runtime ownership".to_string(), vec!["async".into()])
        .unwrap();

    assert_eq!(service.backfill_note_embeddings().unwrap(), 1);
    let initial = service.search_semantic("tokio runtime", 5).unwrap();
    assert!(initial.iter().any(|note| note.id == original.id));

    let edited = service
        .edit_note(
            &original.id,
            "sql query planner index".to_string(),
            vec!["database".into()],
        )
        .unwrap();

    assert_eq!(service.backfill_note_embeddings().unwrap(), 1);
    let old_results = service.search_semantic("tokio runtime", 5).unwrap();
    assert!(!old_results.iter().any(|note| note.id == edited.id));

    let updated_results = service.search_semantic("sql planner", 5).unwrap();
    assert!(updated_results.iter().any(|note| note.id == edited.id));

    service.delete_note(&edited.id).unwrap();
    let after_delete = service.search_semantic("sql planner", 5).unwrap();
    assert!(!after_delete.iter().any(|note| note.id == edited.id));

    service.restore_note(&edited.id).unwrap();
    assert_eq!(service.backfill_note_embeddings().unwrap(), 1);
    let after_restore = service.search_semantic("sql planner", 5).unwrap();
    assert!(after_restore.iter().any(|note| note.id == edited.id));
}

#[test]
fn test_fusion_search_combines_fuzzy_and_semantic_results() {
    let service = SynapService::new(None).unwrap();

    let rust_note = service
        .create_note(
            "rust async runtime ownership".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();
    service
        .create_note(
            "gardening watering schedule".to_string(),
            vec!["life".into()],
        )
        .unwrap();
    assert_eq!(service.backfill_note_embeddings().unwrap(), 2);

    let results = service
        .search_fusion("async ownership", 5, Some(10), Some(10))
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].note.id, rust_note.id);
    assert!(results[0].score > 0.0);
    assert!(results[0].sources.contains(&SearchSourceDTO::Fuzzy));
    assert!(results[0].sources.contains(&SearchSourceDTO::Semantic));
    assert_eq!(
        results[0].text_match,
        Some(SearchTextMatchDTO::Fuzzy)
    );
    assert_eq!(
        results[0].text_match_ranges,
        Some(vec![
            SearchMatchRangeDTO { start: 5, end: 10 },
            SearchMatchRangeDTO { start: 19, end: 28 },
        ])
    );
}

#[test]
fn test_fusion_search_ranges_reference_original_note_content() {
    let service = SynapService::new(None).unwrap();
    let content = "开头 ![](data:image/png;base64,abc) 之后吧";
    let note = service.create_note(content.to_string(), vec![]).unwrap();
    let start = content[..content.find("之后吧").unwrap()]
        .encode_utf16()
        .count() as u32;

    let results = service
        .search_fusion("之后吧", 5, Some(5), Some(0))
        .unwrap();

    let result = results
        .iter()
        .find(|result| result.note.id == note.id)
        .expect("note should be returned by fuzzy search");
    assert_eq!(
        result.text_match_ranges,
        Some(vec![SearchMatchRangeDTO {
            start,
            end: start + 3,
        }])
    );
}

#[test]
fn test_backfill_note_embeddings_fills_pending_slots() {
    let service = SynapService::new(None).unwrap();

    let note = service
        .create_note("pending embedding note".to_string(), vec!["async".into()])
        .unwrap();

    let before = service.search_semantic("pending embedding", 5).unwrap();
    assert!(!before.iter().any(|item| item.id == note.id));

    assert_eq!(service.backfill_note_embeddings().unwrap(), 1);
    assert_eq!(service.backfill_note_embeddings().unwrap(), 0);

    let after = service.search_semantic("pending embedding", 5).unwrap();
    assert!(after.iter().any(|item| item.id == note.id));
}

#[test]
fn test_stale_backfill_snapshot_cannot_restore_superseded_vector() {
    let service = SynapService::new(None).unwrap();
    let original = service
        .create_note("old embedding text".to_string(), vec![])
        .unwrap();
    let original_id = Uuid::parse_str(&original.id).unwrap();
    let stale_text = service
        .with_read(|_tx, reader| Ok(reader.get_by_id(&original_id)?.unwrap().get_search_text()))
        .unwrap();

    let edited = service
        .edit_note(&original.id, "new embedding text".to_string(), vec![])
        .unwrap();
    let committed = service
        .commit_note_embedding(original_id, &stale_text, &vec![1.0; 384])
        .unwrap();

    assert!(!committed);
    let old_vector = service
        .with_read(|tx, _reader| service.semantic_index.get(tx, original_id.as_bytes()))
        .unwrap();
    assert!(old_vector.is_none());

    let edited_id = Uuid::parse_str(&edited.id).unwrap();
    let edited_vector = service
        .with_read(|tx, _reader| service.semantic_index.get(tx, edited_id.as_bytes()))
        .unwrap()
        .unwrap();
    assert!(edited_vector.is_empty());
}

struct BlockingEmbedding {
    started: mpsc::Sender<()>,
    release: Arc<(StdMutex<bool>, Condvar)>,
}

impl EmbeddingModel for BlockingEmbedding {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let _ = self.started.send(());
        let (released, wake) = &*self.release;
        let mut released = released.lock().unwrap();
        while !*released {
            released = wake.wait(released).unwrap();
        }
        Ok(vec![1.0; 384])
    }

    fn dimension(&self) -> usize {
        384
    }
}

#[test]
fn test_slow_backfill_does_not_hold_write_transaction_and_serializes_config_change() {
    let service = Arc::new(SynapService::new(None).unwrap());
    service
        .create_note("slow remote embedding".to_string(), vec![])
        .unwrap();

    let (started_tx, started_rx) = mpsc::channel();
    let release = Arc::new((StdMutex::new(false), Condvar::new()));
    service
        .semantic_index
        .set_embedding_model(Arc::new(BlockingEmbedding {
            started: started_tx,
            release: release.clone(),
        }));

    let backfill_service = service.clone();
    let backfill = thread::spawn(move || backfill_service.backfill_note_embeddings());
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let (config_tx, config_rx) = mpsc::channel();
    let config_service = service.clone();
    let config_change = thread::spawn(move || {
        let result = config_service.set_embedding_config(EmbeddingConfigDTO {
            provider: EmbeddingProviderDTO::LocalHash,
            dimension: 256,
            endpoint: None,
            api_key: None,
            model: None,
            timeout_ms: None,
        });
        let _ = config_tx.send(result);
    });
    assert!(config_rx.recv_timeout(Duration::from_millis(100)).is_err());

    let (create_tx, create_rx) = mpsc::channel();
    let create_service = service.clone();
    let create_note = thread::spawn(move || {
        let result = create_service.create_note("write stays available".to_string(), vec![]);
        let _ = create_tx.send(result);
    });
    let create_completed_while_embedding = create_rx.recv_timeout(Duration::from_secs(1));

    let (released, wake) = &*release;
    *released.lock().unwrap() = true;
    wake.notify_all();

    create_note.join().unwrap();
    assert!(create_completed_while_embedding.unwrap().is_ok());
    assert_eq!(backfill.join().unwrap().unwrap(), 1);
    config_change.join().unwrap();
    assert_eq!(config_rx.recv().unwrap().unwrap().dimension, 256);
}

#[test]
fn test_embedding_config_default_and_change_invalidates_vectors() {
    let service = SynapService::new(None).unwrap();

    let cfg = service.get_embedding_config();
    assert_eq!(cfg.provider, EmbeddingProviderDTO::LocalHash);
    assert_eq!(cfg.dimension, 384);

    let note = service
        .create_note("config change ownership".to_string(), vec!["cfg".into()])
        .unwrap();
    assert_eq!(service.backfill_note_embeddings().unwrap(), 1);
    assert!(
        service
            .search_semantic("ownership", 5)
            .unwrap()
            .iter()
            .any(|item| item.id == note.id)
    );

    // 改成另一个本地维度：应清空向量
    let updated = service
        .set_embedding_config(EmbeddingConfigDTO {
            provider: EmbeddingProviderDTO::LocalHash,
            dimension: 256,
            endpoint: None,
            api_key: None,
            model: None,
            timeout_ms: None,
        })
        .unwrap();
    assert_eq!(updated.dimension, 256);
    assert_eq!(service.get_embedding_config().dimension, 256);

    let after_invalidate = service.search_semantic("ownership", 5).unwrap();
    assert!(!after_invalidate.iter().any(|item| item.id == note.id));

    let mut progress = Vec::new();
    let filled = service
        .backfill_note_embeddings_with_progress(&mut |p| progress.push(p))
        .unwrap();
    assert_eq!(filled, 1);
    assert!(!progress.is_empty());
    assert_eq!(
        progress.last().unwrap().processed,
        progress.last().unwrap().total
    );
    assert_eq!(progress.last().unwrap().filled, 1);

    assert!(
        service
            .search_semantic("ownership", 5)
            .unwrap()
            .iter()
            .any(|item| item.id == note.id)
    );
}

#[test]
fn test_root_config_api_persists_across_reopen() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let expected = CoreConfig::new(EmbeddingConfig::local_hash(256));

    let service = SynapService::open(&db_path).unwrap();
    assert_eq!(service.set_config(expected.clone()).unwrap(), expected);
    assert_eq!(service.get_config(), expected);
    drop(service);

    let reopened = SynapService::open(&db_path).unwrap();
    assert_eq!(reopened.get_config(), expected);
}

#[test]
fn test_get_all_tags_returns_sorted_contents() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    service
        .create_note(
            "tagged".to_string(),
            vec![
                " rust ".into(),
                "async".into(),
                "python".into(),
                "rust".into(),
            ],
        )
        .unwrap();

    let tags = service.get_all_tags().unwrap();
    assert_eq!(
        tags,
        vec![
            "async".to_string(),
            "python".to_string(),
            "rust".to_string(),
        ]
    );
}

#[test]
fn test_get_notes_by_tag_returns_only_live_latest_matches() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let dropped = service
        .create_note("learn rust".to_string(), vec!["rust".into()])
        .unwrap();
    let _replacement = service
        .edit_note(&dropped.id, "learn async".to_string(), vec!["async".into()])
        .unwrap();

    let deleted = service
        .create_note("ship rust".to_string(), vec!["rust".into()])
        .unwrap();
    service.delete_note(&deleted.id).unwrap();

    let live = service
        .create_note("keep rust".to_string(), vec!["rust".into()])
        .unwrap();

    let rust_notes = service.get_notes_by_tag(" rust ", None, None).unwrap();
    assert_eq!(rust_notes.len(), 1);
    assert_eq!(rust_notes[0].id, live.id);

    assert!(
        service
            .get_notes_by_tag("missing", None, None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_get_notes_by_tag_uses_cursor_pagination() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let first = service
        .create_note("rust first".to_string(), vec!["rust".into()])
        .unwrap();
    let second = service
        .create_note("rust second".to_string(), vec!["rust".into()])
        .unwrap();
    let third = service
        .create_note("rust third".to_string(), vec!["rust".into()])
        .unwrap();

    let page_one = service.get_notes_by_tag("rust", None, Some(2)).unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, first.id);
    assert_eq!(page_one[1].id, second.id);

    let page_two = service
        .get_notes_by_tag("rust", Some(&page_one[1].id), Some(2))
        .unwrap();
    assert_eq!(page_two.len(), 1);
    assert_eq!(page_two[0].id, third.id);
}

#[test]
fn test_get_filtered_notes_keeps_global_time_order() {
    let service = SynapService::new(None).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();
    let fourth = service.create_note("fourth".to_string(), vec![]).unwrap();

    service.delete_note(&second.id).unwrap();
    service.delete_note(&fourth.id).unwrap();

    let filtered = service
        .get_timeline_notes_page(
            vec![],
            true,
            false,
            FilteredNoteStatus::All,
            false,
            None,
            TimelineDirection::Older,
            Some(10),
        )
        .unwrap()
        .notes;

    assert_eq!(
        filtered
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![fourth.id, third.id, second.id, first.id]
    );
}

#[test]
fn test_get_filtered_notes_supports_mixed_tags_and_untagged() {
    let service = SynapService::new(None).unwrap();

    let rust = service
        .create_note("rust".to_string(), vec!["rust".into()])
        .unwrap();
    let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
    let travel = service
        .create_note("travel".to_string(), vec!["travel".into()])
        .unwrap();
    let rust_work = service
        .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
        .unwrap();

    let filtered = service
        .get_timeline_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            false,
            None,
            TimelineDirection::Older,
            Some(10),
        )
        .unwrap()
        .notes;

    assert_eq!(
        filtered
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![rust_work.id, travel.id, untagged.id, rust.id]
    );
}

#[test]
fn test_get_filtered_notes_uses_cursor_after_filtering() {
    let service = SynapService::new(None).unwrap();

    let rust = service
        .create_note("rust".to_string(), vec!["rust".into()])
        .unwrap();
    let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
    let travel = service
        .create_note("travel".to_string(), vec!["travel".into()])
        .unwrap();
    let rust_work = service
        .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
        .unwrap();

    let page_one = service
        .get_timeline_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            false,
            None,
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap()
        .notes;
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, rust_work.id);
    assert_eq!(page_one[1].id, travel.id);

    let page_two = service
        .get_timeline_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            false,
            Some(&page_one[1].id),
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap()
        .notes;
    assert_eq!(page_two.len(), 2);
    assert_eq!(page_two[0].id, untagged.id);
    assert_eq!(page_two[1].id, rust.id);
}

#[test]
fn test_create_note_updates_service_searchers() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let created = service
        .create_note(
            "learn rust ownership".to_string(),
            vec![" rust ".into(), "async".into(), "rust".into(), "".into()],
        )
        .unwrap();

    assert_eq!(created.content, "learn rust ownership");
    assert_eq!(created.tags, vec!["rust".to_string(), "async".to_string()]);

    let note_hits = service.search("ownership", 10).unwrap();
    assert!(note_hits.iter().any(|note| note.id == created.id));

    let tag_hits = service.search_tags("rust", 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "rust"));
}

#[test]
fn test_create_note_exposes_millisecond_timestamp() {
    let service = SynapService::new(None).unwrap();
    let created = service.create_note("timed".to_string(), vec![]).unwrap();

    assert!(created.created_at >= 1_000_000_000_000);
}

#[test]
fn test_edit_note_creates_new_version_and_refreshes_tags() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let created = service
        .create_note("learn rust".to_string(), vec!["rust".into()])
        .unwrap();

    let edited = service
        .edit_note(
            &created.id,
            "learn rust async".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();

    assert_ne!(created.id, edited.id);
    assert_eq!(edited.content, "learn rust async");
    assert_eq!(edited.tags, vec!["rust".to_string(), "async".to_string()]);
    let edited_from = edited
        .edited_from
        .as_ref()
        .expect("edited_from should exist");
    assert_eq!(edited_from.id, created.id);
    assert_eq!(edited_from.content_preview, "learn rust");
    assert!(edited.reply_to.is_none());

    let tag_hits = service.search_tags("async", 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "async"));

    let note_hits = service.search("rust async", 10).unwrap();
    assert!(note_hits.iter().any(|note| note.id == edited.id));
}

#[test]
fn test_reply_note_links_child_and_indexes_tags() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let parent = service
        .create_note("parent".to_string(), vec!["root".into()])
        .unwrap();
    let child = service
        .reply_note(&parent.id, "child".to_string(), vec!["reply".into()])
        .unwrap();
    let reply_to = child.reply_to.as_ref().expect("reply_to should exist");
    assert_eq!(reply_to.id, parent.id);
    assert_eq!(reply_to.content_preview, "parent");
    assert!(child.edited_from.is_none());

    let replies = service.get_replies(&parent.id, None, 10).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].id, child.id);

    let tag_hits = service.search_tags("reply", 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "reply"));
}

#[test]
fn test_edited_reply_exposes_both_relation_briefs() {
    let service = SynapService::new(None).unwrap();

    let parent = service
        .create_note("root parent".to_string(), vec![])
        .unwrap();
    let reply = service
        .reply_note(&parent.id, "first reply".to_string(), vec![])
        .unwrap();
    let edited = service
        .edit_note(&reply.id, "reply revised".to_string(), vec![])
        .unwrap();

    let reply_to = edited.reply_to.as_ref().expect("reply_to should exist");
    let edited_from = edited
        .edited_from
        .as_ref()
        .expect("edited_from should exist");
    assert_eq!(reply_to.id, parent.id);
    assert_eq!(reply_to.content_preview, "root parent");
    assert_eq!(edited_from.id, reply.id);
    assert_eq!(edited_from.content_preview, "first reply");
}

#[test]
fn test_get_recent_note_uses_cursor_pagination() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

    #[allow(deprecated)]
    let page_one = service.get_recent_note(None, Some(2)).unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, third.id);
    assert_eq!(page_one[1].id, second.id);

    #[allow(deprecated)]
    let page_two = service
        .get_recent_note(Some(&page_one[1].id), Some(2))
        .unwrap();
    assert_eq!(page_two.len(), 1);
    assert_eq!(page_two[0].id, first.id);
}

#[test]
fn test_get_recent_notes_page_returns_service_cursor() {
    let service = SynapService::new(None).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

    #[allow(deprecated)]
    let page_one = service
        .get_recent_notes_page(None, TimelineDirection::Older, Some(2))
        .unwrap();

    assert_eq!(
        page_one
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![third.id.clone(), second.id.clone()]
    );
    assert_eq!(page_one.next_cursor.as_deref(), Some(second.id.as_str()));

    #[allow(deprecated)]
    let page_two = service
        .get_recent_notes_page(
            page_one.next_cursor.as_deref(),
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap();

    assert_eq!(page_two.notes.len(), 1);
    assert_eq!(page_two.notes[0].id, first.id);
    assert!(page_two.next_cursor.is_none());
}

#[test]
fn test_get_filtered_notes_page_uses_service_cursor_after_filtering() {
    let service = SynapService::new(None).unwrap();

    let rust = service
        .create_note("rust".to_string(), vec!["rust".into()])
        .unwrap();
    let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
    let travel = service
        .create_note("travel".to_string(), vec!["travel".into()])
        .unwrap();
    let rust_work = service
        .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
        .unwrap();

    #[allow(deprecated)]
    let page_one = service
        .get_filtered_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            None,
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap();

    assert_eq!(
        page_one
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![rust_work.id.clone(), travel.id.clone()]
    );
    assert_eq!(page_one.next_cursor.as_deref(), Some(travel.id.as_str()));

    #[allow(deprecated)]
    let page_two = service
        .get_filtered_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            page_one.next_cursor.as_deref(),
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap();

    assert_eq!(
        page_two
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![untagged.id, rust.id]
    );
    assert!(page_two.next_cursor.is_none());
}

#[test]
fn test_get_timeline_notes_page_unifies_filtering_and_cursor() {
    let service = SynapService::new(None).unwrap();

    let rust = service
        .create_note("rust".to_string(), vec!["rust".into()])
        .unwrap();
    let untagged = service.create_note("untagged".to_string(), vec![]).unwrap();
    let travel = service
        .create_note("travel".to_string(), vec!["travel".into()])
        .unwrap();
    let rust_work = service
        .create_note("rust work".to_string(), vec!["rust".into(), "work".into()])
        .unwrap();

    let page_one = service
        .get_timeline_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            false,
            None,
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap();

    assert_eq!(
        page_one
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![rust_work.id.clone(), travel.id.clone()]
    );
    assert_eq!(page_one.next_cursor.as_deref(), Some(travel.id.as_str()));
    assert!(
        page_one
            .notes
            .iter()
            .all(|note| note.timeline_group.is_none())
    );

    let page_two = service
        .get_timeline_notes_page(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            false,
            page_one.next_cursor.as_deref(),
            TimelineDirection::Older,
            Some(2),
        )
        .unwrap();

    assert_eq!(
        page_two
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![untagged.id, rust.id]
    );
    assert!(page_two.next_cursor.is_none());
}

#[test]
fn test_get_timeline_notes_page_can_attach_session_metadata() {
    let service = SynapService::new(None).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

    let page = service
        .get_timeline_notes_page(
            Vec::new(),
            true,
            false,
            FilteredNoteStatus::Normal,
            true,
            None,
            TimelineDirection::Older,
            Some(10),
        )
        .unwrap();

    assert_eq!(
        page.notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![third.id, second.id, first.id]
    );
    assert!(page.next_cursor.is_none());

    let groups = page
        .notes
        .iter()
        .map(|note| note.timeline_group.as_ref().expect("timeline group"))
        .collect::<Vec<_>>();
    assert!(groups[0].starts_group);
    assert!(!groups[1].starts_group);
    assert!(!groups[2].starts_group);
    assert_eq!(groups[0].note_count, 3);
    assert_eq!(groups[1].started_at, groups[0].started_at);
    assert_eq!(groups[2].ended_at, groups[0].ended_at);
}

#[test]
fn test_get_recent_sessions_returns_hydrated_notes() {
    let service = SynapService::new(None).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

    #[allow(deprecated)]
    let page = service.get_recent_sessions(None, Some(10)).unwrap();

    assert!(page.next_cursor.is_none());
    assert_eq!(page.sessions.len(), 1);
    assert_eq!(page.sessions[0].note_count, 3);
    assert_eq!(
        page.sessions[0]
            .notes
            .iter()
            .map(|note| note.id.clone())
            .collect::<Vec<_>>(),
        vec![third.id, second.id, first.id]
    );
}

#[test]
fn test_get_recent_sessions_filters_deleted_and_superseded_notes() {
    let service = SynapService::new(None).unwrap();

    let original = service.create_note("draft".to_string(), vec![]).unwrap();
    let edited = service
        .edit_note(&original.id, "published".to_string(), vec![])
        .unwrap();
    let deleted = service.create_note("deleted".to_string(), vec![]).unwrap();
    service.delete_note(&deleted.id).unwrap();
    let live = service.create_note("live".to_string(), vec![]).unwrap();

    #[allow(deprecated)]
    let page = service.get_recent_sessions(None, Some(10)).unwrap();
    let notes = &page.sessions[0].notes;

    assert_eq!(page.sessions.len(), 1);
    assert_eq!(page.sessions[0].note_count, 2);
    assert!(notes.iter().any(|note| note.id == edited.id));
    assert!(notes.iter().any(|note| note.id == live.id));
    assert!(!notes.iter().any(|note| note.id == original.id));
    assert!(!notes.iter().any(|note| note.id == deleted.id));
}

#[test]
fn test_get_origins_returns_only_parent_layer() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let root = service.create_note("root".to_string(), vec![]).unwrap();
    let middle = service
        .reply_note(&root.id, "middle".to_string(), vec![])
        .unwrap();
    let leaf = service
        .reply_note(&middle.id, "leaf".to_string(), vec![])
        .unwrap();

    let origins = service.get_origins(&leaf.id).unwrap();
    assert_eq!(origins.len(), 1);
    assert_eq!(origins[0].id, middle.id);
}

#[test]
fn test_get_origins_depth_one_keeps_only_compacted_parent_layer() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let root = service.create_note("root".to_string(), vec![]).unwrap();
    let middle = service
        .reply_note(&root.id, "middle".to_string(), vec![])
        .unwrap();
    let leaf = service
        .reply_note(&middle.id, "leaf".to_string(), vec![])
        .unwrap();

    let origins = service.get_origins(&leaf.id).unwrap();
    assert_eq!(origins.len(), 1);
    assert_eq!(origins[0].id, middle.id);
    assert_ne!(origins[0].id, root.id);
}

#[test]
fn test_version_queries_return_live_related_versions() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let v1 = service
        .create_note("Version 1".to_string(), vec!["alpha".to_string()])
        .unwrap();
    let v2a = service
        .edit_note(
            &v1.id,
            "Version 2A".to_string(),
            vec!["alpha".to_string(), "beta".to_string()],
        )
        .unwrap();
    let v2b = service
        .edit_note(&v1.id, "Version 2B".to_string(), vec!["gamma".to_string()])
        .unwrap();

    let previous = service.get_previous_versions(&v2a.id).unwrap();
    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].note.id, v1.id);
    assert_eq!(previous[0].diff.tags.added, Vec::<String>::new());
    assert_eq!(previous[0].diff.tags.removed, vec!["beta"]);
    assert!(
        previous[0]
            .diff
            .content
            .iter()
            .any(|change| !matches!(change.kind, crate::dto::NoteTextChangeKindDTO::Equal))
    );

    let next = service.get_next_versions(&v1.id).unwrap();
    assert_eq!(next.len(), 2);
    assert!(next.iter().any(|note| note.note.id == v2a.id));
    assert!(next.iter().any(|note| note.note.id == v2b.id));
    let next_v2a = next.iter().find(|note| note.note.id == v2a.id).unwrap();
    assert_eq!(next_v2a.diff.tags.added, vec!["beta"]);
    assert_eq!(next_v2a.diff.tags.removed, Vec::<String>::new());
    assert!(
        next_v2a
            .diff
            .content
            .iter()
            .any(|change| !matches!(change.kind, crate::dto::NoteTextChangeKindDTO::Equal))
    );

    let others = service.get_other_versions(&v2a.id).unwrap();
    assert_eq!(others.len(), 2);
    assert!(others.iter().any(|note| note.note.id == v1.id));
    assert!(others.iter().any(|note| note.note.id == v2b.id));
}

#[test]
fn test_version_query_diff_stats_count_changed_lines_by_line_diff() {
    let service = SynapService::new(None).unwrap();

    let original = service
        .create_note("alpha\nbeta\ngamma".to_string(), vec![])
        .unwrap();
    let edited = service
        .edit_note(
            &original.id,
            "alpha\nbeta changed\ngamma\ndelta".to_string(),
            vec![],
        )
        .unwrap();

    let next = service.get_next_versions(&original.id).unwrap();
    let version = next.iter().find(|item| item.note.id == edited.id).unwrap();

    assert_eq!(version.diff.content_stats.inserted_lines, 2);
    assert_eq!(version.diff.content_stats.deleted_lines, 1);
    assert!(version.diff.content_stats.inserted_chars > 0);
}

#[test]
fn test_deleted_note_iteration_and_restore_round_trip() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();

    service.delete_note(&first.id).unwrap();
    service.delete_note(&second.id).unwrap();

    assert!(matches!(
        service.get_note(&second.id),
        Err(ServiceError::NotFound(_))
    ));

    let deleted = service.get_deleted_notes(None, Some(2)).unwrap();
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted[0].id, second.id);
    assert_eq!(deleted[1].id, first.id);

    let deleted_page_two = service
        .get_deleted_notes(Some(&deleted[0].id), Some(2))
        .unwrap();
    assert_eq!(deleted_page_two.len(), 1);
    assert_eq!(deleted_page_two[0].id, first.id);

    service.restore_note(&second.id).unwrap();

    let remaining_deleted = service.get_deleted_notes(None, Some(10)).unwrap();
    assert_eq!(remaining_deleted.len(), 1);
    assert_eq!(remaining_deleted[0].id, first.id);

    let restored = service.get_note(&second.id).unwrap();
    assert_eq!(restored.id, second.id);
}

#[test]
fn test_recent_and_search_filter_superseded_versions_and_markdown_media() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let original = service
        .create_note(
            "hello ![cover](data:image/png;base64,AAAA) rust".to_string(),
            vec![],
        )
        .unwrap();
    let edited = service
        .edit_note(
            &original.id,
            "hello ![cover](data:image/png;base64,BBBB) rust async".to_string(),
            vec![],
        )
        .unwrap();

    #[allow(deprecated)]
    let recent = service.get_recent_note(None, Some(10)).unwrap();
    assert!(recent.iter().any(|note| note.id == edited.id));
    assert!(!recent.iter().any(|note| note.id == original.id));

    let rust_hits = service.search("rust", 10).unwrap();
    assert!(rust_hits.iter().any(|note| note.id == edited.id));
    assert!(!rust_hits.iter().any(|note| note.id == original.id));

    let image_hits = service.search("AAAA", 10).unwrap();
    assert!(image_hits.is_empty());
}

#[test]
fn test_fusion_search_default_fuzzy_limit_is_not_disabled_by_index_lag() {
    let service = SynapService::open_memory().unwrap();
    let note = service
        .create_note("rust ownership search".into(), vec!["rust".into()])
        .unwrap();

    let results = service
        .search_fusion("ownership", 5, None, Some(0))
        .unwrap();
    let matching = results
        .iter()
        .find(|result| result.note.id == note.id)
        .expect("new note should be fuzzy-searchable immediately");
    assert!(matching.sources.contains(&SearchSourceDTO::Fuzzy));
}

#[test]
fn test_share_export_and_import_are_exposed_via_service() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("share-service-a.redb");
    let path_b = dir.path().join("share-service-b.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let root = service_a
        .create_note("share root".to_string(), vec!["rust".into()])
        .unwrap();
    let reply = service_a
        .reply_note(&root.id, "share child".to_string(), vec!["thread".into()])
        .unwrap();

    let exported = service_a
        .export_share(&vec![root.id.clone(), reply.id.clone()])
        .unwrap();
    assert!(!exported.is_empty());

    let stats = service_b.import_share(&exported).unwrap();
    assert_eq!(stats.records, 2);
    assert_eq!(stats.records_applied, 2);
    assert_eq!(stats.bytes, exported.len() as u64);

    let imported_root = service_b.get_note(&root.id).unwrap();
    let imported_reply = service_b.get_note(&reply.id).unwrap();
    assert_eq!(imported_root.content, "share root");
    assert_eq!(imported_reply.content, "share child");
}

#[test]
fn test_export_share_rejects_invalid_note_ids() {
    let service = SynapService::new(None).unwrap();

    let err = service
        .export_share(&vec!["bad-id".to_string()])
        .unwrap_err();
    assert!(matches!(
        err,
        ServiceError::InvalidId | ServiceError::UuidErr(_)
    ));
}

#[test]
fn test_relay_fetch_updates_returns_stats_dto() {
    let sender = SynapService::open_memory().unwrap();
    let recipient = SynapService::open_memory().unwrap();
    let sender_identity = sender.get_local_identity().unwrap();
    let sender_ed25519: [u8; 32] = sender_identity
        .signing
        .public_key
        .as_slice()
        .try_into()
        .unwrap();
    recipient
        .trust_peer(&sender_ed25519, Some("trusted-sender".into()))
        .unwrap();

    sender
        .create_note("relay fetched note".to_owned(), vec!["relay".to_owned()])
        .unwrap();
    let sender_inventory = sender.build_relay_inventory().unwrap();
    let share = sender
        .export_relay_share_for_inventory(&crate::sync::RelayInventory {
            version: crate::sync::RelayInventory::VERSION,
            records: Vec::new(),
        })
        .unwrap();
    let envelope_bytes = relay_sync_test_envelope(
        &sender,
        recipient.local_relay_mailbox_public_key().unwrap(),
        crate::sync::RelaySyncEnvelope {
            inventory: sender_inventory,
            share,
        },
    );

    let server = multi_request_server(vec![
        FakeResponse::mailbox_ok(hex::encode(sender_ed25519), "lease-1", envelope_bytes),
        FakeResponse::ack_no_content(),
        FakeResponse::mailbox_empty(),
    ]);

    let stats = recipient
        .relay_fetch_updates(&server.base_url(), None)
        .unwrap();

    assert_eq!(stats.fetched_messages, 1);
    assert_eq!(stats.imported_messages, 1);
    assert_eq!(stats.dropped_untrusted_messages, 0);
    assert_eq!(stats.acked_messages, 1);
}

#[test]
fn test_relay_push_updates_returns_stats_dto() {
    let service = SynapService::open_memory().unwrap();
    let peer_without_cache = SynapService::open_memory().unwrap();
    let peer_with_cache = SynapService::open_memory().unwrap();

    let peer_without_cache_key = peer_without_cache.local_relay_mailbox_public_key().unwrap();
    let peer_with_cache_key = peer_with_cache.local_relay_mailbox_public_key().unwrap();
    service
        .trust_peer(&peer_without_cache_key, Some("peer-without-cache".into()))
        .unwrap();
    service
        .trust_peer(&peer_with_cache_key, Some("peer-with-cache".into()))
        .unwrap();

    let cached_inventory = peer_with_cache.build_relay_inventory().unwrap();
    service
        .cache_relay_peer_inventory(&peer_with_cache_key, cached_inventory, 111)
        .unwrap();

    service
        .create_note("push relay note".to_owned(), vec!["relay".to_owned()])
        .unwrap();

    let server = multi_request_server(vec![FakeResponse::accepted(), FakeResponse::accepted()]);
    let stats = service
        .relay_push_updates(&server.base_url(), None)
        .unwrap();

    assert_eq!(stats.trusted_peers, 2);
    assert_eq!(stats.posted_messages, 2);
    assert_eq!(stats.full_sync_messages, 1);
    assert_eq!(stats.incremental_sync_messages, 1);
}

struct FakeServer {
    addr: String,
    done_rx: mpsc::Receiver<()>,
}

impl FakeServer {
    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        let _ = self.done_rx.recv_timeout(Duration::from_secs(1));
    }
}

fn multi_request_server(responses: Vec<FakeResponse>) -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let (done_tx, done_rx) = mpsc::channel();

    thread::spawn(move || {
        for response in responses {
            let (stream, _) = listener.accept().unwrap();
            read_http_request(&stream);
            write_http_response(stream, &response.bytes);
        }
        let _ = done_tx.send(());
    });

    FakeServer { addr, done_rx }
}

struct FakeResponse {
    bytes: Vec<u8>,
}

impl FakeResponse {
    fn accepted() -> Self {
        Self {
            bytes: b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n".to_vec(),
        }
    }

    fn ack_no_content() -> Self {
        Self {
            bytes: b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n".to_vec(),
        }
    }

    fn mailbox_empty() -> Self {
        let body = r#"{"code":"mailbox_empty","error":"mailbox is empty"}"#;
        Self {
            bytes: format!(
                "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
            .into_bytes(),
        }
    }

    fn mailbox_ok(sender_ed25519_hex: String, lease_id: &str, body: Vec<u8>) -> Self {
        let mut response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\nx-synap-sender-ed25519: {}\r\nx-synap-lease-id: {}\r\nx-synap-leased-until: 1711111111\r\ncontent-length: {}\r\n\r\n",
            sender_ed25519_hex,
            lease_id,
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(&body);
        Self { bytes: response }
    }
}

fn write_http_response(mut stream: std::net::TcpStream, bytes: &[u8]) {
    use std::io::Write;
    let _ = stream.write_all(bytes);
    let _ = stream.flush();
}

fn read_http_request(mut stream: &std::net::TcpStream) -> String {
    read_http_request_with_body(&mut stream).0
}

fn read_http_request_with_body(mut stream: &std::net::TcpStream) -> (String, Vec<u8>) {
    use std::io::Read;

    let mut header_bytes = Vec::new();
    let mut buf = [0u8; 1];
    while stream.read(&mut buf).ok() == Some(1) {
        header_bytes.push(buf[0]);
        if header_bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let headers = String::from_utf8_lossy(&header_bytes);
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length: ")
                .or_else(|| line.strip_prefix("Content-Length: "))
        })
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = vec![0u8; content_length];
    let _ = stream.read_exact(&mut body);
    (headers.into_owned(), body)
}

fn relay_sync_test_envelope(
    sender: &SynapService,
    recipient_mailbox_public_key: [u8; 32],
    payload: crate::sync::RelaySyncEnvelope,
) -> Vec<u8> {
    let bytes = postcard::to_allocvec(&payload).unwrap();
    sender
        .seal_relay_payload_for(recipient_mailbox_public_key, &bytes)
        .unwrap()
}

#[test]
fn test_deleted_note_tag_excluded_after_import() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("import-test-a.redb");
    let path_b = dir.path().join("import-test-b.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    // Create a note with a tag on service_a
    let note = service_a
        .create_note("test note".to_string(), vec!["test-tag".into()])
        .unwrap();

    // Verify tag exists
    let tags = service_a.get_all_tags().unwrap();
    assert!(tags.contains(&"test-tag".to_string()));

    // Delete the note
    service_a.delete_note(&note.id).unwrap();

    // Verify tag is excluded after deletion
    let tags_after_delete = service_a.get_all_tags().unwrap();
    assert!(!tags_after_delete.contains(&"test-tag".to_string()));

    // Export the note (including tombstone)
    let exported = service_a.export_share(&vec![note.id.clone()]).unwrap();

    // Import into service_b
    let stats = service_b.import_share(&exported).unwrap();
    assert_eq!(stats.records_applied, 1);

    // Verify tag is NOT shown on service_b (it should be excluded because the note is deleted)
    let tags_on_b = service_b.get_all_tags().unwrap();
    assert!(
        !tags_on_b.contains(&"test-tag".to_string()),
        "Tag 'test-tag' should not appear after importing a deleted note, but got: {:?}",
        tags_on_b
    );
}

#[test]
fn test_get_all_tags_excludes_imported_deleted_notes_during_transaction() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("import-atomic-a.redb");
    let path_b = dir.path().join("import-atomic-b.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    // Create a note with a tag on service_a, then delete it
    let note = service_a
        .create_note("to be deleted".to_string(), vec!["ephemeral".into()])
        .unwrap();
    service_a.delete_note(&note.id).unwrap();

    // Verify on service_a: tag should not appear
    assert!(
        !service_a
            .get_all_tags()
            .unwrap()
            .contains(&"ephemeral".to_string())
    );

    // Export and import
    let exported = service_a.export_share(&vec![note.id.clone()]).unwrap();
    service_b.import_share(&exported).unwrap();

    // Tag should not appear in get_all_tags
    let tags = service_b.get_all_tags().unwrap();
    assert!(
        !tags.contains(&"ephemeral".to_string()),
        "Tag should not appear for deleted note, got: {:?}",
        tags
    );

    // Also verify via search_tags
    let search_results = service_b.search_tags("ephemeral", 10).unwrap();
    assert!(
        !search_results.contains(&"ephemeral".to_string()),
        "Tag should not appear in search_tags for deleted note, got: {:?}",
        search_results
    );
}

#[test]
fn test_set_note_color_exposes_color_and_hides_metadata_from_tags() {
    let service = SynapService::open_memory().unwrap();
    let created = service
        .create_note("colored".into(), vec!["rust".into(), "$price".into()])
        .unwrap();

    assert!(created.color.is_none());
    assert_eq!(created.tags, vec!["rust".to_string(), "$price".to_string()]);

    let colored = service
        .set_note_color(&created.id, Some("#ff0000".into()))
        .unwrap();
    assert_eq!(colored.color.as_deref(), Some("#ff0000"));
    assert_eq!(colored.tags, vec!["rust".to_string(), "$price".to_string()]);
    assert!(
        !colored
            .tags
            .iter()
            .any(|t| t.contains("color") || t.starts_with("$ff"))
    );

    // metadata color tags must not leak into public tag lists
    let all_tags = service.get_all_tags().unwrap();
    assert!(all_tags.contains(&"rust".to_string()));
    assert!(all_tags.contains(&"$price".to_string()));
    assert!(
        !all_tags
            .iter()
            .any(|t| t.starts_with("$color") || t == "$ff0000")
    );

    // edit preserves existing color metadata while updating display tags
    let edited = service
        .edit_note(&colored.id, "colored v2".into(), vec!["rust".into()])
        .unwrap();
    assert_eq!(edited.color.as_deref(), Some("#ff0000"));
    assert_eq!(edited.tags, vec!["rust".to_string()]);

    let cleared = service.set_note_color(&edited.id, None).unwrap();
    assert!(cleared.color.is_none());
    assert_eq!(cleared.tags, vec!["rust".to_string()]);
}

#[test]
fn test_legacy_color_tag_is_read_and_upgraded_on_set() {
    use crate::models::note::Note;
    use crate::models::tag::TagWriter;
    use redb::Database;
    use tempfile::NamedTempFile;

    // Seed a note with legacy Android color tag `$00ff00` directly in storage.
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_string_lossy().into_owned();
    {
        let db = Database::create(temp.path()).unwrap();
        let tx = db.begin_write().unwrap();
        Note::init_schema(&tx).unwrap();
        TagWriter::init_schema(&tx).unwrap();
        let writer = TagWriter::new(&tx);
        let tags = vec![
            writer.find_or_create("work").unwrap(),
            writer.find_or_create("$00ff00").unwrap(),
        ];
        Note::create(&tx, "legacy color".into(), tags).unwrap();
        tx.commit().unwrap();
    }

    let service = SynapService::new(Some(path)).unwrap();
    let notes = service.get_recent_note(None, Some(10)).unwrap();
    let note = notes.into_iter().next().expect("seeded note");
    assert_eq!(note.color.as_deref(), Some("#00ff00"));
    assert_eq!(note.tags, vec!["work".to_string()]);

    let upgraded = service
        .set_note_color(&note.id, Some("#0000ff".into()))
        .unwrap();
    assert_eq!(upgraded.color.as_deref(), Some("#0000ff"));
    assert_eq!(upgraded.tags, vec!["work".to_string()]);
}

#[test]
fn test_draft_commit_create_reply_and_edit_with_color() {
    let service = SynapService::open_memory().unwrap();

    // Create via draft buffer: content + tags + color in one commit.
    let mut draft = service.draft_new().unwrap();
    draft = service
        .draft_update(
            &draft.id,
            Some("root note".into()),
            Some(vec!["rust".into(), "$price".into()]),
            Some(Some("#123456".into())),
            None,
            None,
        )
        .unwrap();
    assert_eq!(draft.color.as_deref(), Some("#123456"));
    assert!(draft.reply_to.is_none());
    assert!(draft.edited_from.is_none());

    let root = service.draft_commit(&draft.id).unwrap();
    assert_eq!(root.content, "root note");
    assert_eq!(root.tags, vec!["rust".to_string(), "$price".to_string()]);
    assert_eq!(root.color.as_deref(), Some("#123456"));
    assert!(service.get_previous_versions(&root.id).unwrap().is_empty());
    assert!(service.draft_list().unwrap().is_empty());

    // Reply draft: pointer + body committed together.
    let reply_draft = service.draft_reply_to(&root.id).unwrap();
    assert_eq!(reply_draft.reply_to.as_deref(), Some(root.id.as_str()));
    service
        .draft_update(
            &reply_draft.id,
            Some("child reply".into()),
            Some(vec!["thread".into()]),
            None,
            None,
            None,
        )
        .unwrap();
    let child = service.draft_commit(&reply_draft.id).unwrap();
    assert_eq!(child.content, "child reply");
    assert_eq!(
        child.reply_to.as_ref().map(|b| b.id.as_str()),
        Some(root.id.as_str())
    );

    let replies = service.get_replies(&root.id, None, 10).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].id, child.id);

    // Edit draft: preserves color unless changed; single commit creates edit edge.
    let edit_draft = service.draft_from_note(&root.id).unwrap();
    assert_eq!(edit_draft.edited_from.as_deref(), Some(root.id.as_str()));
    assert_eq!(edit_draft.color.as_deref(), Some("#123456"));
    service
        .draft_update(
            &edit_draft.id,
            Some("root note v2".into()),
            Some(vec!["rust".into()]),
            Some(Some("#00ff00".into())),
            None,
            None,
        )
        .unwrap();
    let edited = service.draft_commit(&edit_draft.id).unwrap();
    assert_eq!(edited.content, "root note v2");
    assert_eq!(edited.color.as_deref(), Some("#00ff00"));
    assert_eq!(edited.tags, vec!["rust".to_string()]);
    assert_eq!(
        edited.edited_from.as_ref().map(|b| b.id.as_str()),
        Some(root.id.as_str())
    );
    let root_next = service.get_next_versions(&root.id).unwrap();
    assert_eq!(root_next.len(), 1);
    assert_eq!(root_next[0].note.id, edited.id);

    let clear_draft = service.draft_from_note(&edited.id).unwrap();
    service
        .draft_update(
            &clear_draft.id,
            Some("root note v3".into()),
            None,
            Some(None),
            None,
            None,
        )
        .unwrap();
    let cleared = service.draft_commit(&clear_draft.id).unwrap();
    assert!(cleared.color.is_none());
    assert_eq!(
        cleared.edited_from.as_ref().map(|brief| brief.id.as_str()),
        Some(edited.id.as_str())
    );
    let edited_next = service.get_next_versions(&edited.id).unwrap();
    assert_eq!(edited_next.len(), 1);
    assert_eq!(edited_next[0].note.id, cleared.id);
}

#[test]
fn test_draft_discard_and_not_found() {
    let service = SynapService::open_memory().unwrap();
    let draft = service.draft_new().unwrap();
    service.draft_discard(&draft.id).unwrap();
    assert!(matches!(
        service.draft_get(&draft.id),
        Err(ServiceError::DraftNotFound(_))
    ));
    assert!(matches!(
        service.draft_commit(&draft.id),
        Err(ServiceError::DraftNotFound(_))
    ));
}

#[test]
fn test_memory_draft_does_not_enter_note_or_derived_lifecycles_before_commit() {
    let service = SynapService::open_memory().unwrap();
    let draft = service.draft_new().unwrap();
    service
        .draft_update(
            &draft.id,
            Some("in progress searchable text".into()),
            Some(vec!["draft-only-tag".into()]),
            Some(Some("#123456".into())),
            None,
            None,
        )
        .unwrap();

    assert!(service.get_recent_note(None, Some(10)).unwrap().is_empty());
    assert!(service.search("searchable", 10).unwrap().is_empty());
    assert!(
        !service
            .get_all_tags()
            .unwrap()
            .contains(&"draft-only-tag".to_string())
    );

    let draft_uuid = Uuid::parse_str(&draft.id).unwrap();
    let draft_key = *draft_uuid.as_bytes();
    service
        .with_read(|tx, _reader| {
            assert!(service.semantic_index.get(tx, &draft_key)?.is_none());
            Ok(())
        })
        .unwrap();
    assert!(service.build_relay_inventory().unwrap().records.is_empty());

    let persisted = service.draft_persist(&draft.id).unwrap();
    assert!(persisted.persisted);
    assert!(service.get_recent_note(None, Some(10)).unwrap().is_empty());
    assert!(service.search("searchable", 10).unwrap().is_empty());
    assert!(
        !service
            .get_all_tags()
            .unwrap()
            .contains(&"draft-only-tag".to_string())
    );
    service
        .with_read(|tx, _reader| {
            assert!(service.semantic_index.get(tx, &draft_key)?.is_none());
            Ok(())
        })
        .unwrap();
    assert!(service.build_relay_inventory().unwrap().records.is_empty());
}

#[test]
fn test_invalid_memory_draft_update_is_atomic() {
    let service = SynapService::open_memory().unwrap();
    let draft = service.draft_new().unwrap();
    let original = service
        .draft_update(
            &draft.id,
            Some("original".into()),
            Some(vec!["stable".into()]),
            Some(Some("#123456".into())),
            None,
            None,
        )
        .unwrap();

    assert!(
        service
            .draft_update(
                &draft.id,
                Some("must roll back".into()),
                Some(vec!["must-roll-back".into()]),
                Some(Some("not-a-color".into())),
                None,
                None,
            )
            .is_err()
    );

    let current = service.draft_get(&draft.id).unwrap();
    assert_eq!(current.content, original.content);
    assert_eq!(current.tags, original.tags);
    assert_eq!(current.color, original.color);
}

#[test]
fn test_persisted_draft_survives_reopen_and_tracks_revision() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("draft-lifecycle.redb");
    let draft_id = {
        let service = SynapService::open(&path).unwrap();
        let draft = service.draft_new().unwrap();
        service
            .draft_update(
                &draft.id,
                Some("durable content".into()),
                Some(vec!["durable".into()]),
                None,
                None,
                None,
            )
            .unwrap();
        let persisted = service.draft_persist(&draft.id).unwrap();
        assert!(persisted.persisted);
        assert_eq!(persisted.revision, 1);
        persisted.id
    };

    let service = SynapService::open(&path).unwrap();
    let loaded = service.draft_get(&draft_id).unwrap();
    assert!(loaded.persisted);
    assert_eq!(loaded.content, "durable content");
    assert_eq!(loaded.revision, 1);
    let updated = service
        .draft_update_checked(
            &draft_id,
            Some("durable content v2".into()),
            None,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert!(matches!(
        service.draft_update_checked(
            &draft_id,
            Some("stale overwrite".into()),
            None,
            None,
            None,
            None,
            Some(1),
        ),
        Err(ServiceError::DraftRevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(
        service.draft_get(&draft_id).unwrap().content,
        "durable content v2"
    );
}

#[test]
fn test_invalid_commit_preserves_memory_and_persisted_drafts() {
    let service = SynapService::open_memory().unwrap();
    let memory = service.draft_new().unwrap();
    assert!(matches!(
        service.draft_commit(&memory.id),
        Err(ServiceError::InvalidDraft(_))
    ));
    assert!(!service.draft_get(&memory.id).unwrap().persisted);

    let persisted = service.draft_persist(&memory.id).unwrap();
    assert!(matches!(
        service.draft_commit(&persisted.id),
        Err(ServiceError::InvalidDraft(_))
    ));
    assert!(service.draft_get(&persisted.id).unwrap().persisted);
}

#[test]
fn test_draft_commit_is_idempotent_and_persisted_commit_is_atomic() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("draft-commit.redb");
    let draft_id;
    let note_id;
    {
        let service = SynapService::open(&path).unwrap();
        let draft = service.draft_new().unwrap();
        service
            .draft_update(
                &draft.id,
                Some("commit once".into()),
                Some(vec!["once".into()]),
                Some(Some("#abcdef".into())),
                None,
                None,
            )
            .unwrap();
        draft_id = service.draft_persist(&draft.id).unwrap().id;
        let committed = service.draft_commit(&draft_id).unwrap();
        note_id = committed.id.clone();
        assert_eq!(service.draft_commit(&draft_id).unwrap().id, note_id);
        assert!(matches!(
            service.draft_get(&draft_id),
            Err(ServiceError::DraftNotFound(_))
        ));
        assert_eq!(service.get_recent_note(None, Some(10)).unwrap().len(), 1);
    }

    let service = SynapService::open(&path).unwrap();
    assert_eq!(service.draft_commit(&draft_id).unwrap().id, note_id);
    assert_eq!(service.get_recent_note(None, Some(10)).unwrap().len(), 1);
}

#[test]
fn test_concurrent_memory_draft_commit_appends_once() {
    let service = Arc::new(SynapService::open_memory().unwrap());
    let draft = service.draft_new().unwrap();
    service
        .draft_update(
            &draft.id,
            Some("concurrent commit".into()),
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let handles = (0..8)
        .map(|_| {
            let service = Arc::clone(&service);
            let draft_id = draft.id.clone();
            thread::spawn(move || service.draft_commit(&draft_id).unwrap().id)
        })
        .collect::<Vec<_>>();
    let ids = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), 1);
    assert_eq!(service.get_recent_note(None, Some(10)).unwrap().len(), 1);
}
