use std::collections::HashSet;

use synap_core::{NoteDTO, ServiceError, SynapService};

const PAGE_SIZE: usize = 100;

pub fn extract_tags(content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();

    for token in content.split_whitespace() {
        let Some(tag) = token.strip_prefix('#') else {
            continue;
        };

        let tag = tag.trim_matches(|c: char| {
            matches!(
                c,
                ',' | '.' | '!' | '?' | ';' | ':' | '，' | '。' | '！' | '？' | '；' | '：'
            )
        });

        if tag.is_empty() {
            continue;
        }

        let normalized = tag.to_string();
        if seen.insert(normalized.clone()) {
            tags.push(normalized);
        }
    }

    tags
}

pub fn fetch_all_recent(service: &SynapService) -> Result<Vec<NoteDTO>, ServiceError> {
    let mut all = Vec::new();
    let mut cursor = None;

    loop {
        let page = service.get_recent_note(cursor.as_deref(), Some(PAGE_SIZE))?;
        let page_len = page.len();
        if page_len == 0 {
            break;
        }

        cursor = page.last().map(|note| note.id.clone());
        all.extend(page);

        if page_len < PAGE_SIZE {
            break;
        }
    }

    Ok(all)
}

pub fn fetch_all_deleted(service: &SynapService) -> Result<Vec<NoteDTO>, ServiceError> {
    let mut all = Vec::new();
    let mut cursor = None;

    loop {
        let page = service.get_deleted_notes(cursor.as_deref(), Some(PAGE_SIZE))?;
        let page_len = page.len();
        if page_len == 0 {
            break;
        }

        cursor = page.last().map(|note| note.id.clone());
        all.extend(page);

        if page_len < PAGE_SIZE {
            break;
        }
    }

    Ok(all)
}

pub fn fetch_all_notes_by_tag(
    service: &SynapService,
    tag: &str,
) -> Result<Vec<NoteDTO>, ServiceError> {
    let mut all = Vec::new();
    let mut cursor = None;

    loop {
        let page = service.get_notes_by_tag(tag, cursor.as_deref(), Some(PAGE_SIZE))?;
        let page_len = page.len();
        if page_len == 0 {
            break;
        }

        cursor = page.last().map(|note| note.id.clone());
        all.extend(page);

        if page_len < PAGE_SIZE {
            break;
        }
    }

    Ok(all)
}

pub fn fetch_all_replies(
    service: &SynapService,
    parent_id: &str,
) -> Result<Vec<NoteDTO>, ServiceError> {
    let mut all = Vec::new();
    let mut cursor = None;

    loop {
        let page = service.get_replies(parent_id, cursor.clone(), PAGE_SIZE)?;
        let page_len = page.len();
        if page_len == 0 {
            break;
        }

        cursor = page.last().map(|note| note.id.clone());
        all.extend(page);

        if page_len < PAGE_SIZE {
            break;
        }
    }

    Ok(all)
}

pub fn build_reply_tree(
    service: &SynapService,
    root: NoteDTO,
) -> Result<Vec<(NoteDTO, usize)>, ServiceError> {
    let mut graph = Vec::new();
    let mut visited = HashSet::new();
    walk_replies(service, root, 0, &mut visited, &mut graph)?;
    Ok(graph)
}

fn walk_replies(
    service: &SynapService,
    note: NoteDTO,
    depth: usize,
    visited: &mut HashSet<String>,
    graph: &mut Vec<(NoteDTO, usize)>,
) -> Result<(), ServiceError> {
    if !visited.insert(note.id.clone()) {
        return Ok(());
    }

    graph.push((note.clone(), depth));
    for child in fetch_all_replies(service, &note.id)? {
        walk_replies(service, child, depth + 1, visited, graph)?;
    }

    Ok(())
}

pub fn resolve_note_prefix(
    service: &SynapService,
    query: &str,
) -> Result<NoteDTO, Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        return Err("笔记 ID 不能为空".into());
    }

    if matches!(query.len(), 8 | 32 | 36) {
        if let Ok(note) = service.get_note(query) {
            return Ok(note);
        }
    }

    let matches: Vec<_> = fetch_all_recent(service)?
        .into_iter()
        .filter(|note| id_matches(&note.id, query))
        .collect();

    match matches.len() {
        0 => Err(format!("未找到匹配的笔记: {}", query).into()),
        1 => Ok(matches.into_iter().next().expect("single match")),
        count => Err(format!("前缀 '{}' 匹配 {} 个笔记，请提供更多字符", query, count).into()),
    }
}

pub fn latest_note(service: &SynapService) -> Result<NoteDTO, Box<dyn std::error::Error>> {
    service
        .get_recent_note(None, Some(1))?
        .into_iter()
        .next()
        .ok_or_else(|| "账本尚无笔记".into())
}

pub fn add_tag_to_note(note: &NoteDTO, tag: &str) -> Vec<String> {
    let tag = tag.trim();
    if tag.is_empty() {
        return note.tags.clone();
    }

    let mut tags = note.tags.clone();
    if !tags.iter().any(|existing| existing == tag) {
        tags.push(tag.to_string());
    }
    tags
}

pub fn remove_tag_from_note(note: &NoteDTO, tag: &str) -> Vec<String> {
    let tag = tag.trim();
    note.tags
        .iter()
        .filter(|existing| existing.as_str() != tag)
        .cloned()
        .collect()
}

fn id_matches(note_id: &str, query: &str) -> bool {
    let note = normalize_id(note_id);
    let query = normalize_id(query);
    !query.is_empty() && note.starts_with(&query)
}

fn normalize_id(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}
