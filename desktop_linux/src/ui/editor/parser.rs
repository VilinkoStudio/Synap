//! Markdown parser — converts raw markdown into a list of MdBlock using pulldown-cmark.
//!
//! Strategy: use pulldown-cmark's offset_iter to get byte ranges, then extract
//! the raw markdown text directly from the source string. This preserves all
//! original formatting exactly (no lossy reconstruction).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::model::{BlockKind, ListItem, MdBlock};

/// Parse a markdown string into a list of blocks.
pub fn parse_markdown(input: &str) -> Vec<MdBlock> {
    if input.trim().is_empty() {
        return vec![MdBlock {
            kind: BlockKind::Blank,
            source_start: 0,
            source_end: input.len(),
        }];
    }

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(input, options);
    let mut collector = BlockCollector::new(input);
    collector.collect(parser);
    collector.finish()
}

/// Extract raw text from the source at the given byte range.
fn raw(input: &str, start: usize, end: usize) -> String {
    let s = input.get(start..end).unwrap_or("");
    // Trim trailing newlines that pulldown-cmark includes in block ranges
    s.trim_end_matches('\n').to_string()
}

/// Internal state machine that groups parser events into blocks.
struct BlockCollector<'a> {
    input: &'a str,
    blocks: Vec<MdBlock>,
    /// For the current heading: (level, start_offset)
    heading: Option<(u8, usize)>,
    /// For the current paragraph: start offset
    para_start: Option<usize>,
    /// For code blocks: start offset, language, accumulated code text
    code: Option<(usize, Option<String>)>,
    code_buf: String,
    /// For blockquotes: start offset, accumulated text
    quote_start: Option<usize>,
    quote_events: Vec<CapturedEvent>,
    /// For lists: start offset, is_ordered, items
    list: Option<(usize, bool, Vec<ListItem>)>,
    /// Current list item: start offset
    item_start: Option<usize>,
    item_events: Vec<CapturedEvent>,
    /// Pending task list marker state for the current item
    item_checked: Option<bool>,
}

/// Tracks inline events for reconstructing item text.
#[derive(Clone, Debug)]
enum CapturedEvent {
    Text(String),
    SoftBreak,
    HardBreak,
}

impl<'a> BlockCollector<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            blocks: Vec::new(),
            heading: None,
            para_start: None,
            code: None,
            code_buf: String::new(),
            quote_start: None,
            quote_events: Vec::new(),
            list: None,
            item_start: None,
            item_events: Vec::new(),
            item_checked: None,
        }
    }

    fn collect(&mut self, parser: Parser<'a>) {
        for (event, range) in parser.into_offset_iter() {
            match event {
                // ── Headings ──
                Event::Start(Tag::Heading { level, .. }) => {
                    self.flush_quote();
                    self.flush_para();
                    let lv = heading_level(level);
                    self.heading = Some((lv, range.start));
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, start)) = self.heading.take() {
                        // Extract the heading text: skip the leading "# " markers + whitespace
                        let prefix_end = self.input[start..]
                            .find(|c: char| !c.is_whitespace() && c != '#')
                            .map(|i| start + i)
                            .unwrap_or(range.end);
                        let text = raw(self.input, prefix_end, range.end);
                        self.blocks.push(MdBlock {
                            kind: BlockKind::Heading { level, text },
                            source_start: start,
                            source_end: range.end,
                        });
                    }
                }

                // ── Code blocks ──
                Event::Start(Tag::CodeBlock(kind)) => {
                    self.flush_quote();
                    self.flush_para();
                    self.flush_list();
                    let lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(info) => {
                            if info.is_empty() {
                                None
                            } else {
                                Some(info.to_string())
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                    self.code = Some((range.start, lang));
                    self.code_buf.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((start, lang)) = self.code.take() {
                        let code = self.code_buf.trim_end_matches('\n').to_string();
                        self.blocks.push(MdBlock {
                            kind: BlockKind::CodeBlock {
                                language: lang,
                                code,
                            },
                            source_start: start,
                            source_end: range.end,
                        });
                        self.code_buf.clear();
                    }
                }

                // ── Blockquotes ──
                Event::Start(Tag::BlockQuote(_)) => {
                    self.flush_para();
                    self.flush_list();
                    if self.quote_start.is_none() {
                        self.quote_start = Some(range.start);
                        self.quote_events.clear();
                    }
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    // Don't flush here — consecutive blockquotes are merged.
                    // The quote will be flushed when a non-quote block starts.
                }

                // ── Lists ──
                Event::Start(Tag::List(ordered)) => {
                    self.flush_quote();
                    self.flush_para();
                    if self.list.is_none() {
                        self.list = Some((range.start, ordered.is_some(), Vec::new()));
                    }
                }
                Event::End(TagEnd::List(_)) => {
                    self.flush_list();
                }

                Event::Start(Tag::Item) => {
                    self.flush_item();
                    self.item_start = Some(range.start);
                    self.item_events.clear();
                }
                Event::End(TagEnd::Item) => {
                    self.flush_item();
                }

                // ── Horizontal rule ──
                Event::Rule => {
                    self.flush_quote();
                    self.flush_para();
                    self.flush_list();
                    self.blocks.push(MdBlock {
                        kind: BlockKind::HorizontalRule,
                        source_start: range.start,
                        source_end: range.end,
                    });
                }

                // ── Paragraphs ──
                Event::Start(Tag::Paragraph) => {
                    if self.para_start.is_none() {
                        self.para_start = Some(range.start);
                    }
                }
                Event::End(TagEnd::Paragraph) => {
                    self.flush_para();
                }

                // ── Text events ──
                Event::Text(text) => {
                    if self.code.is_some() {
                        self.code_buf.push_str(&text);
                    } else if self.item_start.is_some() {
                        self.item_events
                            .push(CapturedEvent::Text(text.to_string()));
                    } else if self.quote_start.is_some() {
                        self.quote_events
                            .push(CapturedEvent::Text(text.to_string()));
                    }
                }

                Event::SoftBreak => {
                    if self.item_start.is_some() {
                        self.item_events.push(CapturedEvent::SoftBreak);
                    } else if self.quote_start.is_some() {
                        self.quote_events.push(CapturedEvent::SoftBreak);
                    }
                }
                Event::HardBreak => {
                    if self.item_start.is_some() {
                        self.item_events.push(CapturedEvent::HardBreak);
                    } else if self.quote_start.is_some() {
                        self.quote_events.push(CapturedEvent::HardBreak);
                    }
                }

                // ── Task list marker ──
                Event::TaskListMarker(checked) => {
                    self.item_checked = Some(checked);
                }

                _ => {}
            }
        }

        // Flush remaining
        self.flush_para();
        self.flush_list();
    }

    fn flush_quote(&mut self) {
        if let Some(start) = self.quote_start.take() {
            // Reconstruct quote text from events
            let mut text = String::new();
            for ev in self.quote_events.drain(..) {
                match ev {
                    CapturedEvent::Text(t) => text.push_str(&t),
                    CapturedEvent::SoftBreak => text.push('\n'),
                    CapturedEvent::HardBreak => text.push_str("  \n"),
                }
            }
            let text = text.trim().to_string();
            if !text.is_empty() {
                // Add "> " prefix to each line for proper markdown representation
                let quoted = text
                    .lines()
                    .map(|l| format!("> {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.blocks.push(MdBlock {
                    kind: BlockKind::Blockquote(quoted),
                    source_start: start,
                    source_end: start + text.len(),
                });
            }
        }
    }

    fn flush_para(&mut self) {
        if let Some(start) = self.para_start.take() {
            // Find the end: the last byte before the next block element
            let text = self.trimmed_para_text(start);
            if !text.is_empty() {
                let end = start + text.len();
                self.blocks.push(MdBlock {
                    kind: BlockKind::Paragraph(text),
                    source_start: start,
                    source_end: end,
                });
            }
        }
    }

    /// Extract paragraph text from source, preserving inline markdown.
    /// Stops at the first blank line or block-level element.
    fn trimmed_para_text(&self, start: usize) -> String {
        let remaining = &self.input[start..];
        let mut end = remaining.len();

        // Stop at double newline (blank line = paragraph boundary)
        if let Some(pos) = remaining.find("\n\n") {
            end = pos;
        }

        // Also stop at lines that start with block-level markers
        let mut line_start = 0;
        for line in remaining[..end].lines() {
            let trimmed = line.trim_start();
            // Check for block-level starts
            if trimmed.starts_with('#')
                && trimmed
                    .chars()
                    .nth(trimmed.chars().take_while(|c| *c == '#').count())
                    .map_or(true, |c| c.is_whitespace())
            {
                end = line_start;
                break;
            }
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                end = line_start;
                break;
            }
            if trimmed.starts_with("> ") || trimmed == ">" {
                end = line_start;
                break;
            }
            if trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
            {
                // Could be a list item — but only if at line start (not mid-paragraph)
                if line_start == 0 && start > 0 {
                    // Not at document start, might be mid-paragraph
                } else if line_start > 0 {
                    end = line_start;
                    break;
                }
            }
            if trimmed.starts_with("---") || trimmed.starts_with("***") || trimmed.starts_with("___")
            {
                // Could be HR
                let chars: Vec<char> = trimmed.chars().collect();
                if chars.len() >= 3 && chars.iter().all(|c| *c == chars[0] || *c == ' ')
                    && (chars[0] == '-' || chars[0] == '*' || chars[0] == '_')
                {
                    end = line_start;
                    break;
                }
            }
            line_start += line.len() + 1; // +1 for \n
        }

        remaining[..end].trim().to_string()
    }

    fn flush_item(&mut self) {
        if let Some(_start) = self.item_start.take() {
            let text = self.reconstruct_item_text();
            let checked = self.item_checked.take();
            if let Some((_, _, ref mut items)) = self.list {
                items.push(ListItem { text, checked });
            }
        }
    }

    /// Reconstruct list item text from captured events.
    fn reconstruct_item_text(&self) -> String {
        let mut out = String::new();
        for ev in &self.item_events {
            match ev {
                CapturedEvent::Text(t) => out.push_str(t),
                CapturedEvent::SoftBreak => out.push('\n'),
                CapturedEvent::HardBreak => out.push_str("  \n"),
            }
        }
        out
    }

    fn flush_list(&mut self) {
        if let Some((start, is_ordered, items)) = self.list.take() {
            if !items.is_empty() {
                let end = items.last().map(|i| i.text.len()).unwrap_or(0);
                self.blocks.push(MdBlock {
                    kind: if is_ordered {
                        BlockKind::OrderedList(items)
                    } else {
                        BlockKind::BulletList(items)
                    },
                    source_start: start,
                    source_end: start + end, // approximate
                });
            }
        }
    }

    fn finish(mut self) -> Vec<MdBlock> {
        self.flush_quote();
        self.flush_para();
        self.flush_list();
        if self.blocks.is_empty() {
            self.blocks.push(MdBlock {
                kind: BlockKind::Blank,
                source_start: 0,
                source_end: self.input.len(),
            });
        }
        self.blocks
    }
}

fn heading_level(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_heading() {
        let blocks = parse_markdown("# Hello World");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::Heading { level, text } => {
                assert_eq!(*level, 1);
                assert_eq!(text, "Hello World");
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_heading_with_inline() {
        let blocks = parse_markdown("## Hello **bold** world");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::Heading { level, text } => {
                assert_eq!(*level, 2);
                assert_eq!(text, "Hello **bold** world");
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_paragraph_preserves_inline() {
        let blocks = parse_markdown("Hello **world** and *italic*");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::Paragraph(text) => {
                assert_eq!(text, "Hello **world** and *italic*");
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_multi_paragraph() {
        let md = "First paragraph.\n\nSecond paragraph.";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 2);
        match &blocks[0].kind {
            BlockKind::Paragraph(t) => assert_eq!(t, "First paragraph."),
            _ => panic!("expected paragraph 1"),
        }
        match &blocks[1].kind {
            BlockKind::Paragraph(t) => assert_eq!(t, "Second paragraph."),
            _ => panic!("expected paragraph 2"),
        }
    }

    #[test]
    fn parse_bullet_list_dash() {
        let md = "- item one\n- item two\n- item three";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::BulletList(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].text, "item one");
                assert_eq!(items[1].text, "item two");
                assert_eq!(items[2].text, "item three");
            }
            _ => panic!("expected bullet list"),
        }
    }

    #[test]
    fn parse_bullet_list_star() {
        let md = "* alpha\n* beta\n* gamma";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::BulletList(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].text, "alpha");
                assert_eq!(items[1].text, "beta");
                assert_eq!(items[2].text, "gamma");
            }
            _ => panic!("expected bullet list, got {:?}", blocks[0].kind),
        }
    }

    #[test]
    fn parse_task_list() {
        let md = "- [x] done\n- [ ] todo";
        let blocks = parse_markdown(md);
        match &blocks[0].kind {
            BlockKind::BulletList(items) => {
                assert_eq!(items[0].checked, Some(true));
                assert_eq!(items[0].text, "done");
                assert_eq!(items[1].checked, Some(false));
                assert_eq!(items[1].text, "todo");
            }
            _ => panic!("expected task list"),
        }
    }

    #[test]
    fn parse_code_block() {
        let md = "```rust\nfn main() {\n    println!(\"hi\");\n}\n```";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert!(code.contains("fn main()"));
                assert!(code.contains("println!"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_blockquote() {
        // pulldown-cmark may produce 1 or 2 blocks for multi-line quotes
        let md = "> This is a quote\n> with multiple lines";
        let blocks = parse_markdown(md);
        let quote_texts: Vec<String> = blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::Blockquote(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        let combined = quote_texts.join("\n");
        assert!(combined.contains("This is a quote"), "got: {}", combined);
        assert!(combined.contains("multiple lines"), "got: {}", combined);
    }

    #[test]
    fn parse_mixed_document() {
        let md = "# Title\n\nSome text\n\n```python\nprint(1)\n```\n\n- item 1\n- item 2";
        let blocks = parse_markdown(md);
        assert!(blocks.len() >= 4, "expected >= 4 blocks, got {}", blocks.len());
    }

    #[test]
    fn parse_hr() {
        let md = "text\n\n---\n\nmore text";
        let blocks = parse_markdown(md);
        assert!(blocks.len() >= 3);
        assert!(matches!(blocks[1].kind, BlockKind::HorizontalRule));
    }

    #[test]
    fn parse_ordered_list() {
        let md = "1. first\n2. second\n3. third";
        let blocks = parse_markdown(md);
        match &blocks[0].kind {
            BlockKind::OrderedList(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].text, "first");
            }
            _ => panic!("expected ordered list"),
        }
    }
}
