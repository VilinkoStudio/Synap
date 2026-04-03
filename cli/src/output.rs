//! Output formatting and colored display for Synap CLI.

use chrono::{DateTime, Local};
use colored::*;
use synap_core::NoteDTO;

#[derive(Debug, Clone)]
pub struct CliStats {
    pub total_live_notes: usize,
    pub total_deleted_notes: usize,
    pub total_tags: usize,
    pub top_tags: Vec<(String, usize)>,
}

pub fn format_timestamp(timestamp_ms: u64) -> String {
    let secs = (timestamp_ms / 1000) as i64;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;

    let utc = DateTime::from_timestamp(secs, nanos).unwrap_or(DateTime::UNIX_EPOCH);
    let local: DateTime<Local> = utc.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        tags.iter()
            .map(|tag| format!("#{}", tag))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn format_note_list(notes: &[NoteDTO]) {
    if notes.is_empty() {
        println!("\n{}", "还没有笔记。".dimmed());
        return;
    }

    println!();
    for note in notes {
        println!(
            "[{}] {} {}",
            short_id(&note.id).cyan(),
            format_timestamp(note.created_at).dimmed(),
            preview(&note.content, 72)
        );

        if !note.tags.is_empty() {
            println!("    {}", format_tags(&note.tags).cyan());
        }
    }
    println!();
}

pub fn format_note_detail(note: &NoteDTO) {
    println!();
    println!("ID: {}", note.id.cyan());
    println!("创建时间: {}", format_timestamp(note.created_at).dimmed());

    if !note.tags.is_empty() {
        println!("标签: {}", format_tags(&note.tags).cyan());
    }

    println!();
    println!("{}", "内容".bold());
    println!("{}", note.content);
    println!();
}

pub fn format_graph(graph: &[(NoteDTO, usize)]) {
    if graph.is_empty() {
        println!("\n{}", "图谱为空".dimmed());
        return;
    }

    println!();
    for (note, depth) in graph {
        let indent = "  ".repeat(*depth);
        println!(
            "{}[{}] {}",
            indent.dimmed(),
            short_id(&note.id).cyan(),
            preview(&note.content, 64)
        );

        if !note.tags.is_empty() {
            println!("{}    {}", indent.dimmed(), format_tags(&note.tags).cyan());
        }
    }
    println!();
}

pub fn format_stats(stats: &CliStats) {
    println!();
    println!("存活笔记: {}", stats.total_live_notes.to_string().cyan());
    println!(
        "已删除笔记: {}",
        stats.total_deleted_notes.to_string().cyan()
    );
    println!("标签总数: {}", stats.total_tags.to_string().cyan());

    if !stats.top_tags.is_empty() {
        println!();
        println!("{}", "热门标签".bold());
        for (tag, count) in &stats.top_tags {
            println!("#{} {}", tag.cyan(), count.to_string().dimmed());
        }
    }
    println!();
}

pub fn success(msg: &str) {
    println!("{}", msg.green().bold());
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn preview(content: &str, max_chars: usize) -> String {
    if content.chars().count() > max_chars {
        format!(
            "{}...",
            content
                .chars()
                .take(max_chars.saturating_sub(3))
                .collect::<String>()
        )
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tags() {
        let tags = vec!["rust".to_string(), "programming".to_string()];
        let formatted = format_tags(&tags);
        assert!(formatted.contains("#rust"));
        assert!(formatted.contains("#programming"));
    }

    #[test]
    fn test_format_tags_empty() {
        let tags: Vec<String> = vec![];
        let formatted = format_tags(&tags);
        assert!(formatted.is_empty());
    }
}
