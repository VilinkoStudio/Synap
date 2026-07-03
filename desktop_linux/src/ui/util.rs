//! Pure utility functions extracted from app logic.
//!
//! These functions have no GTK side effects and operate only on strings/data.

/// Render markdown content for reading view — converts block markers to display symbols.
pub fn render_reading_text(content: &str) -> String {
    let rendered = content
        .lines()
        .map(render_reading_line)
        .collect::<Vec<_>>()
        .join("\n");

    if rendered.trim().is_empty() {
        "空白笔记".to_string()
    } else {
        rendered
    }
}

/// Parse comma-separated tag text into a deduplicated Vec.
pub fn parse_tags(raw: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in raw.split([',', '，']) {
        let trimmed = tag.trim();
        if trimmed.is_empty() || tags.iter().any(|existing: &String| existing == trimmed) {
            continue;
        }
        tags.push(trimmed.to_string());
    }
    tags
}

/// Compact content to a single line with max chars, adding "..." if truncated.
pub fn compact_single_line(content: &str, max_chars: usize) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "空白笔记".to_string();
    }
    if normalized.chars().count() <= max_chars {
        normalized
    } else {
        let preview: String = normalized.chars().take(max_chars).collect();
        format!("{preview}...")
    }
}

/// Render a single markdown line for the reading view.
fn render_reading_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if trimmed.is_empty() {
        return String::new();
    }

    let (prefix, mut text) = if let Some(rest) = trimmed.strip_prefix("> ") {
        ("│ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        ("☐ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        ("☑ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- ") {
        ("• ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("* ") {
        ("• ", rest)
    } else {
        ("", trimmed)
    };

    while let Some(rest) = text.strip_prefix('#') {
        text = rest.trim_start();
    }

    format!("{indent}{prefix}{}", strip_inline_markdown(text))
}

/// Strip inline markdown markers for plain-text display.
fn strip_inline_markdown(text: &str) -> String {
    text.replace("***", "")
        .replace("**", "")
        .replace('*', "")
        .replace("~~", "")
        .replace("==", "")
        .replace("<u>", "")
        .replace("</u>", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tags_basic() {
        let tags = parse_tags("rust, idea, diary");
        assert_eq!(tags, vec!["rust", "idea", "diary"]);
    }

    #[test]
    fn parse_tags_chinese_comma() {
        let tags = parse_tags("rust，idea，diary");
        assert_eq!(tags, vec!["rust", "idea", "diary"]);
    }

    #[test]
    fn parse_tags_dedup() {
        let tags = parse_tags("rust, rust, idea");
        assert_eq!(tags, vec!["rust", "idea"]);
    }

    #[test]
    fn parse_tags_empty() {
        let tags = parse_tags("");
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tags_whitespace() {
        let tags = parse_tags("  rust  ,  idea  ");
        assert_eq!(tags, vec!["rust", "idea"]);
    }

    #[test]
    fn compact_single_line_short() {
        let result = compact_single_line("hello world", 100);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn compact_single_line_long() {
        let result = compact_single_line("a very long text that goes on and on", 10);
        assert_eq!(result, "a very lon...");
    }

    #[test]
    fn compact_single_line_empty() {
        let result = compact_single_line("", 100);
        assert_eq!(result, "空白笔记");
    }

    #[test]
    fn render_reading_text_heading() {
        let result = render_reading_text("# Hello");
        assert!(result.contains("Hello"));
        assert!(!result.contains("#"));
    }

    #[test]
    fn render_reading_text_bullet() {
        let result = render_reading_text("- item one\n- item two");
        assert!(result.contains("• item one"));
        assert!(result.contains("• item two"));
    }

    #[test]
    fn render_reading_text_checkbox() {
        let result = render_reading_text("- [x] done\n- [ ] todo");
        assert!(result.contains("☑ done"));
        assert!(result.contains("☐ todo"));
    }

    #[test]
    fn render_reading_text_blockquote() {
        let result = render_reading_text("> quoted text");
        assert!(result.contains("│ quoted text"));
    }

    #[test]
    fn render_reading_text_empty() {
        let result = render_reading_text("");
        assert_eq!(result, "空白笔记");
    }

    #[test]
    fn strip_inline_markdown_bold() {
        assert_eq!(strip_inline_markdown("**bold**"), "bold");
    }

    #[test]
    fn strip_inline_markdown_italic() {
        assert_eq!(strip_inline_markdown("*italic*"), "italic");
    }
}
