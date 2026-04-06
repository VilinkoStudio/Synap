use tempfile::tempdir;
use uuid::Uuid;

use crate::{
    models::note::{EditLinkRecord, ReplyLinkRecord},
    service::SynapService,
    sync::ShareService,
};

use super::share::SharePackage;

#[test]
fn test_export_records_notes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");

    let service = SynapService::new(Some(path.to_string_lossy().into_owned())).unwrap();

    let note1 = service.create_note("note 1".to_string(), vec![]).unwrap();
    let note2 = service.create_note("note 2".to_string(), vec![]).unwrap();

    let note1_id = Uuid::parse_str(&note1.id).unwrap();
    let note2_id = Uuid::parse_str(&note2.id).unwrap();

    let share_service = ShareService::new(&service);
    let records = share_service.export_records(&[note1_id, note2_id]).unwrap();

    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| record.notes.len() == 1));
}

#[test]
fn test_export_records_includes_tags_and_edit_history() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");

    let service = SynapService::new(Some(path.to_string_lossy().into_owned())).unwrap();

    let original = service
        .create_note("original".to_string(), vec!["rust".into()])
        .unwrap();
    let edited = service
        .edit_note(
            &original.id,
            "edited".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();

    let original_id = Uuid::parse_str(&original.id).unwrap();
    let edited_id = Uuid::parse_str(&edited.id).unwrap();

    let share_service = ShareService::new(&service);
    let records = share_service.export_records(&[original_id]).unwrap();

    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.id, original_id);
    assert_eq!(record.notes.len(), 2);
    assert_eq!(
        record.edit_links,
        vec![EditLinkRecord {
            previous_id: original_id,
            next_id: edited_id,
        }]
    );

    let mut tag_names: Vec<_> = record.tags.iter().map(|tag| tag.content.as_str()).collect();
    tag_names.sort_unstable();
    assert_eq!(tag_names, vec!["async", "rust"]);
}

#[test]
fn test_export_records_includes_incident_reply_links() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");

    let service = SynapService::new(Some(path.to_string_lossy().into_owned())).unwrap();

    let parent = service.create_note("parent".to_string(), vec![]).unwrap();
    let reply = service
        .reply_note(&parent.id, "reply".to_string(), vec![])
        .unwrap();

    let parent_id = Uuid::parse_str(&parent.id).unwrap();
    let reply_id = Uuid::parse_str(&reply.id).unwrap();

    let share_service = ShareService::new(&service);
    let records = share_service.export_records(&[parent_id]).unwrap();

    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.notes.len(), 1);
    assert!(record.reply_links.contains(&ReplyLinkRecord {
        parent_id,
        child_id: reply_id,
    }));
}

#[test]
fn test_export_records_deduplicates_same_logical_note() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");

    let service = SynapService::new(Some(path.to_string_lossy().into_owned())).unwrap();

    let original = service.create_note("note".to_string(), vec![]).unwrap();
    let edited = service
        .edit_note(&original.id, "edited".to_string(), vec![])
        .unwrap();

    let original_id = Uuid::parse_str(&original.id).unwrap();
    let edited_id = Uuid::parse_str(&edited.id).unwrap();

    let share_service = ShareService::new(&service);
    let records = share_service
        .export_records(&[original_id, edited_id])
        .unwrap();

    assert_eq!(records.len(), 1);
}

#[test]
fn test_export_bytes_round_trips_package_codec() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.redb");

    let service = SynapService::new(Some(path.to_string_lossy().into_owned())).unwrap();
    let note = service.create_note("note".to_string(), vec![]).unwrap();
    let note_id = Uuid::parse_str(&note.id).unwrap();

    let share_service = ShareService::new(&service);
    let bytes = share_service.export_bytes(&[note_id]).unwrap();
    let package = SharePackage::decode(&bytes).unwrap();

    assert_eq!(package.version, crate::sync::SHARE_VERSION);
    assert_eq!(package.records.len(), 1);
}

#[test]
fn test_share_import_bytes_round_trips_logical_notes() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("share-a.redb");
    let path_b = dir.path().join("share-b.redb");

    let service_a = SynapService::new(Some(path_a.to_string_lossy().into_owned())).unwrap();
    let service_b = SynapService::new(Some(path_b.to_string_lossy().into_owned())).unwrap();

    let root = service_a
        .create_note("learn rust".to_string(), vec!["rust".into()])
        .unwrap();
    let reply = service_a
        .reply_note(&root.id, "child reply".to_string(), vec!["thread".into()])
        .unwrap();
    let edited = service_a
        .edit_note(
            &root.id,
            "learn rust async".to_string(),
            vec!["rust".into(), "async".into()],
        )
        .unwrap();
    service_a.delete_note(&reply.id).unwrap();

    let root_id = Uuid::parse_str(&root.id).unwrap();
    let reply_id = Uuid::parse_str(&reply.id).unwrap();

    let share_a = ShareService::new(&service_a);
    let share_b = ShareService::new(&service_b);
    let bytes = share_a.export_bytes(&[root_id, reply_id]).unwrap();
    let stats = share_b.import_bytes(&bytes).unwrap();

    assert_eq!(stats.records, 2);
    assert_eq!(stats.applied, 2);
    assert_eq!(stats.bytes, bytes.len());

    let rust_hits = service_b.search("rust async", 10).unwrap();
    assert!(rust_hits.iter().any(|note| note.id == edited.id));

    let next_versions = service_b.get_next_versions(&root.id).unwrap();
    assert_eq!(next_versions.len(), 1);
    assert_eq!(next_versions[0].id, edited.id);

    let tag_hits = service_b.search_tags("async", 10).unwrap();
    assert!(tag_hits.iter().any(|tag| tag == "async"));

    let deleted = service_b.get_deleted_notes(None, Some(10)).unwrap();
    assert!(deleted.iter().any(|note| note.id == reply.id));

    let replies = service_b.get_replies(&root.id, None, 10).unwrap();
    assert!(replies.is_empty());
}
