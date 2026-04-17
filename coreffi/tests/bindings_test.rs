//! Integration tests for FFI bindings.

use tempfile::tempdir;
use uniffi_synap_coreffi::{open, open_memory, FfiError, FilteredNoteStatus};
use uuid::Uuid;

fn sorted_ids(notes: &[uniffi_synap_coreffi::NoteDTO]) -> Vec<String> {
    let mut ids = notes.iter().map(|note| note.id.clone()).collect::<Vec<_>>();
    ids.sort();
    ids
}

#[test]
fn test_open_file_database() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("synap.redb").to_string_lossy().into_owned();

    let service = open(path.clone()).unwrap();
    let notes = service.get_recent_note(None, None).unwrap();
    assert!(notes.is_empty());

    let note = service
        .create_note("Test note".to_string(), vec!["rust".to_string()])
        .unwrap();
    assert_eq!(note.content, "Test note");

    drop(service);
    let reopened = open(path).unwrap();
    let retrieved = reopened.get_note(note.id).unwrap();
    assert_eq!(retrieved.content, "Test note");
    assert_eq!(retrieved.tags, vec!["rust"]);
}

#[test]
fn test_open_memory_database() {
    let service = open_memory().unwrap();
    let notes = service.get_recent_note(None, None).unwrap();
    assert!(notes.is_empty());

    let note = service.create_note("Test".to_string(), vec![]).unwrap();
    assert_eq!(note.content, "Test");
}

#[test]
fn test_error_handling() {
    let service = open_memory().unwrap();

    let invalid = service.get_note("bad-id".to_string());
    assert!(matches!(invalid, Err(FfiError::InvalidId)));

    let missing = service.get_note(Uuid::new_v4().to_string());
    assert!(matches!(missing, Err(FfiError::NotFound)));
}

#[test]
fn test_edit_workflow() {
    let service = open_memory().unwrap();

    let original = service
        .create_note("Original content".to_string(), vec!["draft".to_string()])
        .unwrap();

    let edited = service
        .edit_note(
            original.id.clone(),
            "Edited content".to_string(),
            vec!["published".to_string()],
        )
        .unwrap();

    assert_ne!(edited.id, original.id);
    assert_eq!(edited.content, "Edited content");
    assert_eq!(edited.tags, vec!["published"]);

    let original_reloaded = service.get_note(original.id).unwrap();
    assert_eq!(original_reloaded.content, "Original content");

    let edited_reloaded = service.get_note(edited.id).unwrap();
    assert_eq!(edited_reloaded.content, "Edited content");
}

#[test]
fn test_delete_and_restore_workflow() {
    let service = open_memory().unwrap();

    let first = service
        .create_note("To be deleted".to_string(), vec!["trash".to_string()])
        .unwrap();
    let second = service
        .create_note("To be restored".to_string(), vec!["trash".to_string()])
        .unwrap();

    service.delete_note(first.id.clone()).unwrap();
    service.delete_note(second.id.clone()).unwrap();

    let recent = service.get_recent_note(None, Some(20)).unwrap();
    assert!(recent.is_empty());

    let search_hits = service.search("deleted".to_string(), 10).unwrap();
    assert!(search_hits.is_empty());

    assert!(matches!(
        service.get_note(first.id.clone()),
        Err(FfiError::NotFound)
    ));
    assert!(matches!(
        service.get_note(second.id.clone()),
        Err(FfiError::NotFound)
    ));

    let deleted = service.get_deleted_notes(None, Some(2)).unwrap();
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted[0].id, second.id);
    assert_eq!(deleted[1].id, first.id);

    let deleted_page_two = service
        .get_deleted_notes(Some(deleted[0].id.clone()), Some(10))
        .unwrap();
    assert_eq!(deleted_page_two.len(), 1);
    assert_eq!(deleted_page_two[0].id, first.id);

    service.restore_note(second.id.clone()).unwrap();

    let restored = service.get_note(second.id).unwrap();
    assert_eq!(restored.content, "To be restored");

    let remaining_deleted = service.get_deleted_notes(None, Some(10)).unwrap();
    assert_eq!(remaining_deleted.len(), 1);
    assert_eq!(remaining_deleted[0].id, first.id);
}

#[test]
fn test_reply_and_paging_workflow() {
    let service = open_memory().unwrap();

    let parent = service.create_note("Parent".to_string(), vec![]).unwrap();
    let first = service
        .reply_note(parent.id.clone(), "Child 1".to_string(), vec![])
        .unwrap();
    let second = service
        .reply_note(parent.id.clone(), "Child 2".to_string(), vec![])
        .unwrap();

    let page_one = service.get_replies(parent.id.clone(), None, 1).unwrap();
    assert_eq!(page_one.len(), 1);

    let page_two = service
        .get_replies(parent.id, Some(page_one[0].id.clone()), 10)
        .unwrap();
    assert_eq!(page_two.len(), 1);

    let mut seen = vec![page_one[0].id.clone(), page_two[0].id.clone()];
    seen.sort();

    let mut expected = vec![first.id, second.id];
    expected.sort();

    assert_eq!(seen, expected);
}

#[test]
fn test_origins_and_version_queries_workflow() {
    let service = open_memory().unwrap();

    let root = service.create_note("Root".to_string(), vec![]).unwrap();
    let middle = service
        .reply_note(root.id.clone(), "Middle".to_string(), vec![])
        .unwrap();
    let leaf = service
        .reply_note(middle.id.clone(), "Leaf".to_string(), vec![])
        .unwrap();

    let origins = service.get_origins(leaf.id.clone()).unwrap();
    assert_eq!(origins.len(), 1);
    assert_eq!(origins[0].id, middle.id);

    let v2a = service
        .edit_note(root.id.clone(), "Root v2a".to_string(), vec![])
        .unwrap();
    let v2b = service
        .edit_note(root.id.clone(), "Root v2b".to_string(), vec![])
        .unwrap();

    let previous = service.get_previous_versions(v2a.id.clone()).unwrap();
    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].id, root.id);

    let next = service.get_next_versions(root.id.clone()).unwrap();
    assert_eq!(next.len(), 2);
    let next_ids = sorted_ids(&next);
    assert_eq!(next_ids, sorted_ids(&[v2a.clone(), v2b.clone()]));

    let other_versions = service.get_other_versions(v2a.id).unwrap();
    assert_eq!(other_versions.len(), 2);
    let other_ids = sorted_ids(&other_versions);
    assert_eq!(other_ids, sorted_ids(&[root, v2b]));
}

#[test]
fn test_recent_note_cursor_skips_superseded_versions() {
    let service = open_memory().unwrap();

    let first = service.create_note("first".to_string(), vec![]).unwrap();
    let second = service.create_note("second".to_string(), vec![]).unwrap();
    let third = service.create_note("third".to_string(), vec![]).unwrap();
    let third_edited = service
        .edit_note(third.id, "third edited".to_string(), vec![])
        .unwrap();

    let page_one = service.get_recent_note(None, Some(2)).unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one[0].id, third_edited.id);
    assert_eq!(page_one[1].id, second.id);

    let page_two = service
        .get_recent_note(Some(page_one[1].id.clone()), Some(10))
        .unwrap();
    assert_eq!(page_two.len(), 1);
    assert_eq!(page_two[0].id, first.id);
}

#[test]
fn test_share_export_and_import_round_trip() {
    let source = open_memory().unwrap();
    let target = open_memory().unwrap();

    let root = source
        .create_note("share root".to_string(), vec!["rust".to_string()])
        .unwrap();
    let reply = source
        .reply_note(
            root.id.clone(),
            "share reply".to_string(),
            vec!["thread".to_string()],
        )
        .unwrap();

    let bytes = source
        .export_share(vec![root.id.clone(), reply.id.clone()])
        .unwrap();
    assert!(!bytes.is_empty());

    let stats = target.import_share(bytes.clone()).unwrap();
    assert_eq!(stats.records, 2);
    assert_eq!(stats.records_applied, 2);
    assert_eq!(stats.bytes, bytes.len() as u64);

    let imported_root = target.get_note(root.id).unwrap();
    let imported_reply = target.get_note(reply.id).unwrap();
    assert_eq!(imported_root.content, "share root");
    assert_eq!(imported_reply.content, "share reply");
}

#[test]
fn test_export_share_invalid_id_maps_to_ffi_error() {
    let service = open_memory().unwrap();

    let result = service.export_share(vec!["bad-id".to_string()]);
    assert!(matches!(result, Err(FfiError::InvalidId)));
}

#[test]
fn test_recommend_tag_is_exposed() {
    let service = open_memory().unwrap();

    service
        .create_note(
            "Rust ownership and lifetimes for async services".to_string(),
            vec![
                "rust".to_string(),
                "async".to_string(),
                "backend".to_string(),
            ],
        )
        .unwrap();
    service
        .create_note(
            "Tokio runtime, future polling and async scheduling".to_string(),
            vec!["rust".to_string(), "async".to_string()],
        )
        .unwrap();

    let tags = service
        .recommend_tag("tokio async ownership".to_string(), 3)
        .unwrap();
    assert!(tags.iter().any(|tag| tag == "rust"));
    assert!(tags.iter().any(|tag| tag == "async"));
}

#[test]
fn test_search_and_tag_search() {
    let service = open_memory().unwrap();

    let created = service
        .create_note(
            "learn rust ownership".to_string(),
            vec!["rust".to_string(), "async".to_string()],
        )
        .unwrap();

    service
        .create_note("book hotel".to_string(), vec!["travel".to_string()])
        .unwrap();

    let search_hits = service.search("ownership".to_string(), 10).unwrap();
    assert!(search_hits.iter().any(|note| note.id == created.id));

    let recent = service.get_recent_note(None, Some(10)).unwrap();
    assert!(recent.iter().any(|note| note.id == created.id));

    let tag_hits = service.search_tags("rust".to_string(), 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "rust"));
    assert!(tag_hits.iter().all(|tag: &String| !tag.is_empty()));

    let all_tags = service.get_all_tags().unwrap();
    assert_eq!(
        all_tags,
        vec![
            "async".to_string(),
            "rust".to_string(),
            "travel".to_string(),
        ]
    );

    let tag_page_one = service
        .get_notes_by_tag("rust".to_string(), None, Some(1))
        .unwrap();
    assert_eq!(tag_page_one.len(), 1);
    assert_eq!(tag_page_one[0].id, created.id);

    let tag_page_two = service
        .get_notes_by_tag(
            "rust".to_string(),
            Some(tag_page_one[0].id.clone()),
            Some(10),
        )
        .unwrap();
    assert!(tag_page_two.is_empty());

    let missing = service
        .get_notes_by_tag("missing".to_string(), None, None)
        .unwrap();
    assert!(missing.is_empty());

    let filtered = service
        .get_filtered_notes(
            vec!["rust".to_string()],
            false,
            true,
            FilteredNoteStatus::Normal,
            None,
            Some(10),
        )
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, created.id);
}
