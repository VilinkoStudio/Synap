use super::*;
use crate::models::tag::{Tag, TagReader, TagWriter};
use redb::Database;
use redb::ReadableDatabase;
use std::{thread::sleep, time::Duration};
use tempfile::NamedTempFile;

fn create_temp_db() -> Database {
    let temp_file = NamedTempFile::new().unwrap();
    let db = Database::create(temp_file.path()).unwrap();

    let write_txn = db.begin_write().unwrap();
    Note::init_schema(&write_txn).expect("Failed to initialize database schema");
    TagWriter::init_schema(&write_txn).expect("Failed to initialize tag schema");
    write_txn.commit().unwrap();

    db
}

#[test]
fn test_note_create_and_read() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let note = Note::create(&write_txn, "Hello Synap!".to_string(), vec![]).unwrap();

    let note_id = note.get_id();
    let short_id = *note.short_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let found_by_id = reader.get_by_id(&note_id).unwrap().unwrap();
    assert_eq!(found_by_id.content(), "Hello Synap!");

    let found_by_alias = reader.get_by_short_id(&short_id).unwrap().unwrap();
    assert_eq!(found_by_alias.get_id(), note_id);
    assert_eq!(found_by_alias.content(), "Hello Synap!");
}

#[test]
fn test_note_edit_lineage() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![]).unwrap();
    let v1_id = v1.get_id();

    let v2 = v1
        .edit(&write_txn, "Version 2".to_string(), vec![])
        .unwrap();
    let v2_id = v2.get_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let read_v1 = reader.get_by_id(&v1_id).unwrap().unwrap();
    let read_v2 = reader.get_by_id(&v2_id).unwrap().unwrap();

    assert_eq!(read_v1.content(), "Version 1");
    assert_eq!(read_v2.content(), "Version 2");

    let mut next_iters = reader.next_versions(&read_v1).unwrap();
    assert_eq!(next_iters.next().unwrap().unwrap(), v2_id);
    assert!(next_iters.next().is_none());

    let mut prev_iters = reader.previous_versions(&read_v2).unwrap();
    assert_eq!(prev_iters.next().unwrap().unwrap(), v1_id);
}

#[test]
fn test_note_other_versions_walks_entire_edit_component() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![]).unwrap();
    let v1_id = v1.get_id();

    let v2a = v1
        .clone()
        .edit(&write_txn, "Version 2A".to_string(), vec![])
        .unwrap();
    let v2a_id = v2a.get_id();

    let v2b = v1
        .edit(&write_txn, "Version 2B".to_string(), vec![])
        .unwrap();
    let v2b_id = v2b.get_id();

    let v3 = v2a
        .clone()
        .edit(&write_txn, "Version 3".to_string(), vec![])
        .unwrap();
    let v3_id = v3.get_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();
    let read_v2a = reader.get_by_id(&v2a_id).unwrap().unwrap();

    let others: Vec<Uuid> = reader
        .other_versions(&read_v2a)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();

    assert_eq!(others.len(), 3);
    assert!(others.contains(&v1_id));
    assert!(others.contains(&v2b_id));
    assert!(others.contains(&v3_id));
    assert!(!others.contains(&v2a_id));
}

#[test]
fn test_note_topology_links() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();

    let parent = Note::create(&write_txn, "Root Idea".to_string(), vec![]).unwrap();
    let child_1 = Note::create(&write_txn, "Sub Idea 1".to_string(), vec![]).unwrap();
    let child_2 = Note::create(&write_txn, "Sub Idea 2".to_string(), vec![]).unwrap();

    let parent_id = parent.get_id();
    let c1_id = child_1.get_id();
    let c2_id = child_2.get_id();

    parent.reply(&write_txn, &child_1).unwrap();
    child_2.link_to_parent(&write_txn, &parent).unwrap();

    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let read_parent = reader.get_by_id(&parent_id).unwrap().unwrap();
    let read_c1 = reader.get_by_id(&c1_id).unwrap().unwrap();

    let children: Vec<Uuid> = reader
        .children(&read_parent)
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(children.len(), 2);
    assert!(children.contains(&c1_id));
    assert!(children.contains(&c2_id));

    let c1_parents: Vec<Uuid> = reader
        .parents(&read_c1)
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(c1_parents.len(), 1);
    assert_eq!(c1_parents[0], parent_id);
}

#[test]
fn test_note_tag_index_is_append_only_across_edits() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();
    let async_tag = tag_writer.find_or_create("async").unwrap();

    let v1 = Note::create(&write_txn, "learn rust".to_string(), vec![rust.clone()]).unwrap();
    let v1_id = v1.get_id();

    let v2 = v1
        .edit(
            &write_txn,
            "learn rust async".to_string(),
            vec![rust.clone(), async_tag.clone()],
        )
        .unwrap();
    let v2_id = v2.get_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let rust_ids: Vec<Uuid> = reader
        .tagged_note_ids(&rust)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(rust_ids.len(), 2);
    assert!(rust_ids.contains(&v1_id));
    assert!(rust_ids.contains(&v2_id));

    let async_ids: Vec<Uuid> = reader
        .tagged_note_ids(&async_tag)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(async_ids, vec![v2_id]);

    let live_rust_notes: Vec<Uuid> = reader
        .notes_with_tag(&rust)
        .unwrap()
        .map(|res| res.unwrap().get_id())
        .collect();
    assert_eq!(live_rust_notes.len(), 2);
    assert!(live_rust_notes.contains(&v1_id));
    assert!(live_rust_notes.contains(&v2_id));
}

#[test]
fn test_note_tag_index_keeps_tombstoned_entries_but_filters_them_on_read() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();

    let note = Note::create(&write_txn, "ephemeral".to_string(), vec![rust.clone()]).unwrap();
    let note_id = note.get_id();
    note.del(&write_txn).unwrap();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let indexed_ids: Vec<Uuid> = reader
        .tagged_note_ids(&rust)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(indexed_ids, vec![note_id]);

    let visible_notes: Vec<Uuid> = reader
        .notes_with_tag(&rust)
        .unwrap()
        .map(|res| res.unwrap().get_id())
        .collect();
    assert!(visible_notes.is_empty());
}

#[test]
fn test_note_latest_notes_with_tag_filters_superseded_versions() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();
    let async_tag = tag_writer.find_or_create("async").unwrap();

    let v1 = Note::create(&write_txn, "learn rust".to_string(), vec![rust.clone()]).unwrap();
    let _v2 = v1
        .edit(&write_txn, "learn async".to_string(), vec![async_tag])
        .unwrap();
    let live = Note::create(&write_txn, "ship rust".to_string(), vec![rust.clone()]).unwrap();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let visible_notes: Vec<Uuid> = reader
        .latest_notes_with_tag(&rust)
        .unwrap()
        .map(|res| res.unwrap().get_id())
        .collect();
    assert_eq!(visible_notes, vec![live.get_id()]);
}

#[test]
fn test_note_create_deduplicates_repeated_tags() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();

    let note = Note::create(
        &write_txn,
        "dedupe me".to_string(),
        vec![rust.clone(), rust.clone()],
    )
    .unwrap();
    let note_id = note.get_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();
    let stored_note = reader.get_by_id(&note_id).unwrap().unwrap();

    assert_eq!(stored_note.tags().len(), 1);
    assert_eq!(stored_note.tags()[0], rust.get_id());

    let indexed_ids: Vec<Uuid> = reader
        .tagged_note_ids(&rust)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(indexed_ids, vec![note_id]);
}

#[test]
fn test_note_restore_clears_tombstone() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let note = Note::create(&write_txn, "restore me".to_string(), vec![]).unwrap();
    let note_id = note.get_id();
    note.clone().del(&write_txn).unwrap();
    note.restore(&write_txn).unwrap();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();
    let restored = reader.get_by_id(&note_id).unwrap().unwrap();
    assert!(!restored.is_deleted());

    let deleted_ids: Vec<Uuid> = reader
        .deleted_note_ids()
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert!(deleted_ids.is_empty());
}

#[test]
fn test_export_record_captures_logical_note_payload() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();
    let async_tag = tag_writer.find_or_create("async").unwrap();

    let parent = Note::create(&write_txn, "parent".to_string(), vec![]).unwrap();
    let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![rust.clone()]).unwrap();
    let v1_id = v1.get_id();
    parent.reply(&write_txn, &v1).unwrap();

    let v2 = v1
        .clone()
        .edit(
            &write_txn,
            "Version 2".to_string(),
            vec![rust.clone(), async_tag.clone()],
        )
        .unwrap();
    let v2_id = v2.get_id();

    let child = Note::create(&write_txn, "child".to_string(), vec![]).unwrap();
    v2.reply(&write_txn, &child).unwrap();
    v2.clone().del(&write_txn).unwrap();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();
    let record = reader.export_record(&v1_id).unwrap().unwrap();

    assert_eq!(record.id, v1_id);
    assert_eq!(record.notes.len(), 2);
    assert_eq!(
        record.notes.iter().map(|note| note.id).collect::<Vec<_>>(),
        vec![v1_id, v2_id]
    );
    let mut tag_names: Vec<_> = record.tags.iter().map(|tag| tag.content.as_str()).collect();
    tag_names.sort_unstable();
    assert_eq!(tag_names, vec!["async", "rust"]);
    assert_eq!(
        record.edit_links,
        vec![EditLinkRecord {
            previous_id: v1_id,
            next_id: v2_id,
        }]
    );
    assert!(record.reply_links.contains(&ReplyLinkRecord {
        parent_id: parent.get_id(),
        child_id: v1_id,
    }));
    assert!(record.reply_links.contains(&ReplyLinkRecord {
        parent_id: v2_id,
        child_id: child.get_id(),
    }));
    assert_eq!(record.tombstones, vec![v2_id]);
}

#[test]
fn test_export_record_sync_id_is_stable_across_versions() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let rust = TagWriter::new(&write_txn).find_or_create("rust").unwrap();
    let v1 = Note::create(&write_txn, "Version 1".to_string(), vec![rust.clone()]).unwrap();
    let v1_id = v1.get_id();
    let v2 = v1
        .edit(&write_txn, "Version 2".to_string(), vec![rust])
        .unwrap();
    let v2_id = v2.get_id();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let record_from_v1 = reader.export_record(&v1_id).unwrap().unwrap();
    let record_from_v2 = reader.export_record(&v2_id).unwrap().unwrap();

    assert_eq!(record_from_v1, record_from_v2);
    assert_eq!(record_from_v1.sync_id().unwrap(), record_from_v2.sync_id().unwrap());
}

#[test]
fn test_import_records_restores_cross_component_topology() {
    let source_db = create_temp_db();

    let source_write_txn = source_db.begin_write().unwrap();
    let tag_writer = TagWriter::new(&source_write_txn);
    let rust = tag_writer.find_or_create("rust").unwrap();
    let thread = tag_writer.find_or_create("thread").unwrap();

    let root = Note::create(
        &source_write_txn,
        "learn rust".to_string(),
        vec![rust.clone()],
    )
    .unwrap();
    let root_id = root.get_id();
    let edited = root
        .clone()
        .edit(
            &source_write_txn,
            "learn rust async".to_string(),
            vec![rust.clone()],
        )
        .unwrap();
    let edited_id = edited.get_id();

    let reply = Note::create(
        &source_write_txn,
        "child reply".to_string(),
        vec![thread.clone()],
    )
    .unwrap();
    let reply_id = reply.get_id();
    root.reply(&source_write_txn, &reply).unwrap();
    reply.clone().del(&source_write_txn).unwrap();
    source_write_txn.commit().unwrap();

    let source_read_txn = source_db.begin_read().unwrap();
    let source_reader = NoteReader::new(&source_read_txn).unwrap();
    let exported = source_reader.export_records(&[root_id, reply_id]).unwrap();
    assert_eq!(exported.len(), 2);

    let target_db = create_temp_db();
    let target_write_txn = target_db.begin_write().unwrap();
    Note::import_records(&target_write_txn, exported).unwrap();
    target_write_txn.commit().unwrap();

    let target_read_txn = target_db.begin_read().unwrap();
    let target_reader = NoteReader::new(&target_read_txn).unwrap();

    let root_note = target_reader.get_by_id(&root_id).unwrap().unwrap();
    let edited_note = target_reader.get_by_id(&edited_id).unwrap().unwrap();
    let reply_note = target_reader.get_by_id(&reply_id).unwrap().unwrap();

    assert_eq!(root_note.content(), "learn rust");
    assert_eq!(edited_note.content(), "learn rust async");
    assert!(reply_note.is_deleted());

    let next_versions: Vec<Uuid> = target_reader
        .next_versions(&root_note)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(next_versions, vec![edited_id]);

    let children: Vec<Uuid> = target_reader
        .children(&root_note)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(children, vec![reply_id]);

    let deleted_ids: Vec<Uuid> = target_reader
        .deleted_note_ids()
        .unwrap()
        .map(|res| res.unwrap())
        .collect();
    assert_eq!(deleted_ids, vec![reply_id]);

    let tag_reader = TagReader::new(&target_read_txn).unwrap();
    let rust_id = Tag::id_for_content("rust").unwrap();
    let thread_id = Tag::id_for_content("thread").unwrap();
    assert_eq!(
        tag_reader
            .get_by_id(&rust_id)
            .unwrap()
            .unwrap()
            .get_content(),
        "rust"
    );
    assert_eq!(
        tag_reader
            .get_by_id(&thread_id)
            .unwrap()
            .unwrap()
            .get_content(),
        "thread"
    );
}

#[test]
fn test_note_by_time_range_respects_uuid_bounds() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let first = Note::create(&write_txn, "first".to_string(), vec![]).unwrap();
    sleep(Duration::from_millis(2));
    let second = Note::create(&write_txn, "second".to_string(), vec![]).unwrap();
    sleep(Duration::from_millis(2));
    let third = Note::create(&write_txn, "third".to_string(), vec![]).unwrap();
    sleep(Duration::from_millis(2));
    let fourth = Note::create(&write_txn, "fourth".to_string(), vec![]).unwrap();
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let reader = NoteReader::new(&read_txn).unwrap();

    let ids: Vec<Uuid> = reader
        .note_by_time_range(
            Bound::Included(second.get_id()),
            Bound::Excluded(fourth.get_id()),
        )
        .unwrap()
        .map(|res| res.unwrap())
        .collect();

    assert_eq!(ids, vec![second.get_id(), third.get_id()]);

    let tail_ids: Vec<Uuid> = reader
        .note_by_time_range(Bound::Excluded(second.get_id()), Bound::Unbounded)
        .unwrap()
        .map(|res| res.unwrap())
        .collect();

    assert_eq!(tail_ids, vec![third.get_id(), fourth.get_id()]);
    assert!(!tail_ids.contains(&first.get_id()));
}

#[test]
fn test_note_search_text_filters_markdown_images_and_data_uris() {
    let db = create_temp_db();

    let write_txn = db.begin_write().unwrap();
    let note = Note::create(
        &write_txn,
        "hello ![cover](data:image/png;base64,AAAA) world data:image/jpeg;base64,BBBB".to_string(),
        vec![],
    )
    .unwrap();
    write_txn.commit().unwrap();

    let search_text = note.get_search_text();
    assert_eq!(search_text, "hello world");
    assert!(!search_text.contains("data:image/"));
    assert!(!search_text.contains("!["));
}
