use crate::{
    core::{CoreResult, DesktopCore},
    domain::{HomeData, NoteDetailData},
};
use synap_core::dto::NoteDTO;

const PAGE_SIZE: usize = 50;

pub fn load_home(core: &dyn DesktopCore, query: &str) -> CoreResult<HomeData> {
    let trimmed = query.trim();

    let notes = if trimmed.is_empty() {
        let page = core.recent_notes_page(None, Some(PAGE_SIZE))?;
        let has_more = page.next_cursor.is_some();
        HomeData {
            notes: page.notes,
            deleted_notes: Vec::new(),
            notes_cursor: page.next_cursor,
            deleted_notes_cursor: None,
            has_more_notes: has_more,
            has_more_deleted_notes: false,
        }
    } else {
        let notes = core.search(trimmed, PAGE_SIZE)?;
        HomeData {
            notes,
            deleted_notes: Vec::new(),
            notes_cursor: None,
            deleted_notes_cursor: None,
            has_more_notes: false,
            has_more_deleted_notes: false,
        }
    };

    let deleted_page = core.deleted_notes_page(None, Some(PAGE_SIZE))?;
    let has_more_deleted = deleted_page.next_cursor.is_some();

    Ok(HomeData {
        notes: notes.notes,
        deleted_notes: deleted_page.notes,
        notes_cursor: notes.notes_cursor,
        deleted_notes_cursor: deleted_page.next_cursor,
        has_more_notes: notes.has_more_notes,
        has_more_deleted_notes: has_more_deleted,
    })
}

pub fn load_more_notes(
    core: &dyn DesktopCore,
    cursor: &str,
) -> CoreResult<(Vec<NoteDTO>, Option<String>, bool)> {
    let page = core.recent_notes_page(Some(cursor), Some(PAGE_SIZE))?;
    let has_more = page.next_cursor.is_some();
    let cursor = page.next_cursor;
    Ok((page.notes, cursor, has_more))
}

pub fn load_more_deleted_notes(
    core: &dyn DesktopCore,
    cursor: &str,
) -> CoreResult<(Vec<NoteDTO>, Option<String>, bool)> {
    let page = core.deleted_notes_page(Some(cursor), Some(PAGE_SIZE))?;
    let has_more = page.next_cursor.is_some();
    let cursor = page.next_cursor;
    Ok((page.notes, cursor, has_more))
}

pub fn load_note_detail(core: &dyn DesktopCore, note_id: &str) -> CoreResult<NoteDetailData> {
    let note = core.get_note(note_id)?;
    let replies = core.replies(note_id, None, 20)?;
    let origins = core.origins(note_id)?;
    let other_versions = core.other_versions(note_id)?;

    Ok(NoteDetailData {
        note,
        replies,
        origins,
        other_versions,
    })
}
