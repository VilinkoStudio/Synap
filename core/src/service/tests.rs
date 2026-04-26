use super::*;
use tempfile::tempdir;

fn seed_db(path: &Path, tags: &[&str]) {
    let db = Database::create(path).unwrap();

    let tx = db.begin_write().unwrap();
    Note::init_schema(&tx).unwrap();
    TagWriter::init_schema(&tx).unwrap();

    let tag_writer = TagWriter::new(&tx);
    for tag in tags {
        tag_writer.find_or_create(*tag).unwrap();
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
fn test_open_existing_db_auto_creates_crypto_schema_and_identity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    seed_db(&db_path, &["rust"]);

    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();
    let local_identity = service.get_local_identity().unwrap();
    assert_eq!(local_identity.identity.public_key.len(), 32);
    assert_eq!(local_identity.signing.public_key.len(), 32);
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

    let initial = service.recommend_tag("tokio runtime", 3).unwrap();
    assert!(initial.iter().any(|tag| tag == "async"));

    let edited = service
        .edit_note(
            &original.id,
            "sql index join planner".to_string(),
            vec!["database".into()],
        )
        .unwrap();

    let updated = service.recommend_tag("sql planner", 3).unwrap();
    assert!(updated.iter().any(|tag| tag == "database"));

    let old_query = service.recommend_tag("tokio runtime", 3).unwrap();
    assert!(!old_query.iter().any(|tag| tag == "async"));

    service.delete_note(&edited.id).unwrap();
    let after_delete = service.recommend_tag("sql planner", 3).unwrap();
    assert!(!after_delete.iter().any(|tag| tag == "database"));

    service.restore_note(&edited.id).unwrap();
    let after_restore = service.recommend_tag("sql planner", 3).unwrap();
    assert!(after_restore.iter().any(|tag| tag == "database"));
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

    let initial = service.search_semantic("tokio runtime", 5).unwrap();
    assert!(initial.iter().any(|note| note.id == original.id));

    let edited = service
        .edit_note(
            &original.id,
            "sql query planner index".to_string(),
            vec!["database".into()],
        )
        .unwrap();

    let old_results = service.search_semantic("tokio runtime", 5).unwrap();
    assert!(!old_results.iter().any(|note| note.id == edited.id));

    let updated_results = service.search_semantic("sql planner", 5).unwrap();
    assert!(updated_results.iter().any(|note| note.id == edited.id));

    service.delete_note(&edited.id).unwrap();
    let after_delete = service.search_semantic("sql planner", 5).unwrap();
    assert!(!after_delete.iter().any(|note| note.id == edited.id));

    service.restore_note(&edited.id).unwrap();
    let after_restore = service.search_semantic("sql planner", 5).unwrap();
    assert!(after_restore.iter().any(|note| note.id == edited.id));
}

#[test]
fn test_get_starmap_returns_latest_visible_notes_only() {
    let service = SynapService::new(None).unwrap();

    let original = service
        .create_note("tokio runtime ownership".to_string(), vec!["async".into()])
        .unwrap();
    let edited = service
        .edit_note(
            &original.id,
            "tokio runtime ownership updated".to_string(),
            vec!["async".into()],
        )
        .unwrap();
    let deleted = service
        .create_note("deleted note".to_string(), vec!["misc".into()])
        .unwrap();
    let live = service
        .create_note("live note".to_string(), vec!["misc".into()])
        .unwrap();

    service.delete_note(&deleted.id).unwrap();

    let points = service.get_starmap().unwrap();
    let ids = points
        .iter()
        .map(|point| point.id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&edited.id.as_str()));
    assert!(ids.contains(&live.id.as_str()));
    assert!(!ids.contains(&original.id.as_str()));
    assert!(!ids.contains(&deleted.id.as_str()));
    assert!(points.iter().all(|point| {
        point.x.is_finite()
            && point.y.is_finite()
            && point.x >= -1.0
            && point.x <= 1.0
            && point.y >= -1.0
            && point.y <= 1.0
    }));
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

    assert!(service
        .get_notes_by_tag("missing", None, None)
        .unwrap()
        .is_empty());
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
        .get_filtered_notes(vec![], true, false, FilteredNoteStatus::All, None, Some(10))
        .unwrap();

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
        .get_filtered_notes(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            None,
            Some(10),
        )
        .unwrap();

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
        .get_filtered_notes(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            None,
            Some(2),
        )
        .unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, rust_work.id);
    assert_eq!(page_one[1].id, travel.id);

    let page_two = service
        .get_filtered_notes(
            vec!["rust".into(), "travel".into()],
            true,
            true,
            FilteredNoteStatus::Normal,
            Some(&page_one[1].id),
            Some(2),
        )
        .unwrap();
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

    let replies = service.get_replies(&parent.id, None, 10).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].id, child.id);

    let tag_hits = service.search_tags("reply", 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "reply"));
}

#[test]
fn test_get_recent_note_uses_cursor_pagination() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("synap.redb");
    let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

    let page_one = service.get_recent_note(None, Some(2)).unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, third.id);
    assert_eq!(page_one[1].id, second.id);

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
fn test_get_recent_sessions_returns_hydrated_notes() {
    let service = SynapService::new(None).unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();

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
        .create_note("Version 1".to_string(), vec![])
        .unwrap();
    let v2a = service
        .edit_note(&v1.id, "Version 2A".to_string(), vec![])
        .unwrap();
    let v2b = service
        .edit_note(&v1.id, "Version 2B".to_string(), vec![])
        .unwrap();

    let previous = service.get_previous_versions(&v2a.id).unwrap();
    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].id, v1.id);

    let next = service.get_next_versions(&v1.id).unwrap();
    assert_eq!(next.len(), 2);
    assert!(next.iter().any(|note| note.id == v2a.id));
    assert!(next.iter().any(|note| note.id == v2b.id));

    let others = service.get_other_versions(&v2a.id).unwrap();
    assert_eq!(others.len(), 2);
    assert!(others.iter().any(|note| note.id == v1.id));
    assert!(others.iter().any(|note| note.id == v2b.id));
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
