use chrono::{Local, TimeZone};

pub fn preview(content: &str, limit: usize) -> String {
    let flattened = content.replace('\n', " ");
    let mut chars = flattened.chars();
    let preview: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

pub fn format_timestamp(timestamp: u64) -> String {
    let ts = timestamp as i64;
    let maybe_time = if ts > 10_000_000_000 {
        Local.timestamp_millis_opt(ts).single()
    } else {
        Local.timestamp_opt(ts, 0).single()
    };

    maybe_time
        .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

pub fn short_note_id(id: &str) -> String {
    id.chars().take(8).collect()
}
