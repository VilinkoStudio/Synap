use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use rusqlite::Connection;
use synap_core::SynapService;

#[derive(Debug, Parser)]
#[command(name = "xiaomi-notes-import")]
#[command(about = "Import Xiaomi Notes SQLite data into a Synap database")]
struct Cli {
    #[arg(long, default_value = "note.db")]
    source: PathBuf,

    #[arg(long, default_value = "synap_database.redb")]
    target: PathBuf,

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    limit: Option<usize>,
}

#[derive(Debug)]
struct XiaomiNote {
    id: i64,
    created_date: u64,
    title: String,
    plain_text: String,
    rich_text: Option<String>,
    attachment_count: usize,
    attachment_mime_types: Vec<String>,
}

#[derive(Debug)]
struct PreparedNote {
    source_id: i64,
    created_at_ms: u64,
    content: String,
}

#[derive(Debug, Default)]
struct ImportStats {
    scanned: usize,
    prepared: usize,
    imported: usize,
    skipped_empty: usize,
    notes_with_attachments: usize,
    attachment_mime_counts: BTreeMap<String, usize>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let source = Connection::open(&cli.source)
        .with_context(|| format!("failed to open Xiaomi Notes db: {}", cli.source.display()))?;
    let source_notes = load_notes(&source, cli.limit)?;

    let mut stats = ImportStats {
        scanned: source_notes.len(),
        ..ImportStats::default()
    };
    let prepared = prepare_notes(source_notes, &mut stats);

    if cli.dry_run {
        print_stats(&stats, true);
        return Ok(());
    }

    let service = SynapService::open(&cli.target).with_context(|| {
        format!(
            "failed to open target Synap db: {}",
            cli.target.as_path().display()
        )
    })?;

    for note in &prepared {
        service
            .create_note_at(note.content.clone(), vec![], note.created_at_ms)
            .with_context(|| format!("failed to import Xiaomi note {}", note.source_id))?;
        stats.imported += 1;
    }

    print_stats(&stats, false);
    Ok(())
}

fn load_notes(conn: &Connection, limit: Option<usize>) -> Result<Vec<XiaomiNote>> {
    let mut sql = String::from(
        "select \
            n._id, \
            n.created_date, \
            coalesce(n.title, '') as title, \
            coalesce(n.plain_text, '') as plain_text, \
            (select d.content from data d \
             where d.note_id = n._id and d.mime_type = 'vnd.android.cursor.item/text_note' \
             order by d._id asc limit 1) as rich_text, \
            (select count(*) from data d \
             where d.note_id = n._id and d.mime_type != 'vnd.android.cursor.item/text_note') as attachment_count, \
            (select group_concat(distinct d.mime_type) from data d \
             where d.note_id = n._id and d.mime_type != 'vnd.android.cursor.item/text_note') as attachment_types \
        from note n \
        where n.type = 0 and n.deletion_tag = 0 \
        order by n.created_date asc, n._id asc",
    );

    if limit.is_some() {
        sql.push_str(" limit ?1");
    }

    let mut stmt = conn.prepare(&sql)?;
    match limit {
        Some(limit) => {
            let rows = stmt.query_map([limit as i64], map_note_row)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        }
        None => {
            let rows = stmt.query_map([], map_note_row)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        }
    }
}

fn map_note_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<XiaomiNote> {
    let attachment_types = row
        .get::<_, Option<String>>(6)?
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    Ok(XiaomiNote {
        id: row.get(0)?,
        created_date: row.get::<_, u64>(1)?,
        title: row.get(2)?,
        plain_text: row.get(3)?,
        rich_text: row.get(4)?,
        attachment_count: row.get::<_, i64>(5)?.max(0) as usize,
        attachment_mime_types: attachment_types,
    })
}

fn prepare_notes(notes: Vec<XiaomiNote>, stats: &mut ImportStats) -> Vec<PreparedNote> {
    let mut prepared = Vec::with_capacity(notes.len());

    for note in notes {
        if note.attachment_count > 0 {
            stats.notes_with_attachments += 1;
            for mime in &note.attachment_mime_types {
                *stats
                    .attachment_mime_counts
                    .entry(mime.clone())
                    .or_default() += 1;
            }
        }

        let content = compose_note_content(&note);
        if content.is_empty() {
            stats.skipped_empty += 1;
            continue;
        }

        prepared.push(PreparedNote {
            source_id: note.id,
            created_at_ms: note.created_date,
            content,
        });
    }

    stats.prepared = prepared.len();
    prepared
}

fn compose_note_content(note: &XiaomiNote) -> String {
    let title = normalize_text(&note.title);
    let body = if !note.plain_text.trim().is_empty() {
        normalize_text(&note.plain_text)
    } else {
        normalize_text(&cleanup_rich_text(
            note.rich_text.as_deref().unwrap_or_default(),
        ))
    };

    match (title.is_empty(), body.is_empty()) {
        (true, true) => String::new(),
        (false, true) => title,
        (true, false) => body,
        (false, false) => {
            if first_nonempty_line(&body) == Some(title.as_str()) {
                body
            } else {
                format!("{title}\n\n{body}")
            }
        }
    }
}

fn cleanup_rich_text(raw: &str) -> String {
    let mut text = raw.replace("\r\n", "\n");

    text = image_marker_re().replace_all(&text, "").into_owned();
    text = sound_tag_re().replace_all(&text, "").into_owned();
    text = checkbox_checked_re()
        .replace_all(&text, "- [x] ")
        .into_owned();
    text = checkbox_unchecked_re()
        .replace_all(&text, "- [ ] ")
        .into_owned();
    text = text_tag_re().replace_all(&text, "").into_owned();
    text = generic_tag_re().replace_all(&text, "").into_owned();

    decode_xml_entities(&text)
}

fn normalize_text(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n");
    let mut lines = Vec::new();
    let mut previous_blank = true;

    for raw_line in normalized.lines() {
        let line = raw_line.trim_end();
        let is_blank = line.trim().is_empty();
        if is_blank {
            if !previous_blank {
                lines.push(String::new());
            }
            previous_blank = true;
            continue;
        }

        lines.push(line.trim().to_string());
        previous_blank = false;
    }

    while matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn first_nonempty_line(text: &str) -> Option<&str> {
    text.lines().find(|line| !line.trim().is_empty())
}

fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn image_marker_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^☺ .*$").unwrap())
}

fn sound_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<sound\b[^>]*/>"#).unwrap())
}

fn checkbox_unchecked_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<input\b[^>]*type="checkbox"[^>]*/>"#).unwrap())
}

fn checkbox_checked_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<input\b[^>]*type="checkbox"[^>]*checked="1"[^>]*/>"#).unwrap())
}

fn text_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"</?text\b[^>]*>").unwrap())
}

fn generic_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"</?[^>\n]+>").unwrap())
}

fn print_stats(stats: &ImportStats, dry_run: bool) {
    if dry_run {
        println!("dry-run completed");
    } else {
        println!("import completed");
    }

    println!("scanned: {}", stats.scanned);
    println!("prepared: {}", stats.prepared);
    println!("imported: {}", stats.imported);
    println!("skipped_empty: {}", stats.skipped_empty);
    println!("notes_with_attachments: {}", stats.notes_with_attachments);

    if !stats.attachment_mime_counts.is_empty() {
        println!("attachment_mime_types:");
        for (mime, count) in &stats.attachment_mime_counts {
            println!("  {mime}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_rich_text_converts_checkboxes_and_removes_image_markers() {
        let raw = r#"<text indent="1">开学</text>
<input type="checkbox" indent="1" level="3" />鞋子
<input type="checkbox" indent="1" level="3" checked="1" />门卡
☺ abcdef<0/></>
<sound fileid="voice.mp3" />"#;

        let cleaned = cleanup_rich_text(raw);

        assert!(cleaned.contains("开学"));
        assert!(cleaned.contains("- [ ] 鞋子"));
        assert!(cleaned.contains("- [x] 门卡"));
        assert!(!cleaned.contains('☺'));
        assert!(!cleaned.contains("sound"));
    }

    #[test]
    fn compose_note_content_avoids_duplicating_title() {
        let note = XiaomiNote {
            id: 1,
            created_date: 1,
            title: "位置".to_string(),
            plain_text: "位置\n速度".to_string(),
            rich_text: None,
            attachment_count: 0,
            attachment_mime_types: Vec::new(),
        };

        assert_eq!(compose_note_content(&note), "位置\n速度");
    }

    #[test]
    fn normalize_text_collapses_extra_blank_lines() {
        assert_eq!(normalize_text("a\n\n\nb\n\n"), "a\n\nb");
    }
}
