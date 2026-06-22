//! Inline markdown → Pango markup converter.
//!
//! Converts raw inline markdown (e.g. `**bold**`, `` `code` ``) into Pango XML
//! markup that GTK Labels can render with proper formatting.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Convert inline markdown text to Pango markup.
///
/// This handles: **bold**, *italic*, ~~strikethrough~~, `code`,
/// [link](url), ==highlight==, and escaped characters.
pub fn inline_to_pango(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(input, options);
    let mut renderer = PangoRenderer::new();
    renderer.render(parser);
    renderer.finish()
}

struct PangoRenderer {
    output: String,
    /// Stack of open tags for proper nesting.
    tag_stack: Vec<PangoTag>,
}

#[derive(Clone)]
enum PangoTag {
    Bold,
    Italic,
    Strikethrough,
    Link,
}

impl PangoRenderer {
    fn new() -> Self {
        Self {
            output: String::new(),
            tag_stack: Vec::new(),
        }
    }

    fn render(&mut self, parser: Parser<'_>) {
        for event in parser {
            match event {
                Event::Text(text) => {
                    self.push_escaped(&text);
                }
                Event::Code(code) => {
                    self.output.push_str("<span font_family=\"monospace\" font_size=\"smaller\" background=\"alpha(currentColor, 0.08)\">");
                    self.push_escaped(&code);
                    self.output.push_str("</span>");
                }
                Event::Start(Tag::Strong) => {
                    self.output.push_str("<b>");
                    self.tag_stack.push(PangoTag::Bold);
                }
                Event::End(TagEnd::Strong) => {
                    self.close_tag(PangoTag::Bold, "</b>");
                }
                Event::Start(Tag::Emphasis) => {
                    self.output.push_str("<i>");
                    self.tag_stack.push(PangoTag::Italic);
                }
                Event::End(TagEnd::Emphasis) => {
                    self.close_tag(PangoTag::Italic, "</i>");
                }
                Event::Start(Tag::Strikethrough) => {
                    self.output.push_str("<s>");
                    self.tag_stack.push(PangoTag::Strikethrough);
                }
                Event::End(TagEnd::Strikethrough) => {
                    self.close_tag(PangoTag::Strikethrough, "</s>");
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    self.output.push_str(
                        "<span foreground=\"#3584e4\" underline=\"single\">",
                    );
                    self.tag_stack.push(PangoTag::Link);
                    let _ = dest_url; // URL available if needed for hover
                }
                Event::End(TagEnd::Link) => {
                    self.close_tag(PangoTag::Link, "</span>");
                }
                Event::SoftBreak => {
                    // Single newline in markdown → render as actual line break
                    self.output.push('\n');
                }
                Event::HardBreak => {
                    // Two spaces + newline → also a line break
                    self.output.push('\n');
                }
                Event::Html(html) => {
                    // Pass through HTML tags that might be Pango-compatible
                    // (e.g. <u>, <sub>, <sup>)
                    let tag = html.trim();
                    if tag == "<u>" {
                        self.output.push_str("<u>");
                    } else if tag == "</u>" {
                        self.output.push_str("</u>");
                    } else if tag == "<sub>" {
                        self.output.push_str("<sub>");
                    } else if tag == "</sub>" {
                        self.output.push_str("</sub>");
                    } else if tag == "<sup>" {
                        self.output.push_str("<sup>");
                    } else if tag == "</sup>" {
                        self.output.push_str("</sup>");
                    }
                    // Ignore other HTML
                }
                _ => {}
            }
        }
    }

    fn close_tag(&mut self, expected: PangoTag, _closing_markup: &str) {
        // Close until we find the matching tag
        while let Some(top) = self.tag_stack.pop() {
            let close = match &top {
                PangoTag::Bold => "</b>",
                PangoTag::Italic => "</i>",
                PangoTag::Strikethrough => "</s>",
                PangoTag::Link => "</span>",
            };
            self.output.push_str(close);
            if std::mem::discriminant(&top) == std::mem::discriminant(&expected) {
                break;
            }
        }
    }

    fn push_escaped(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '&' => self.output.push_str("&amp;"),
                '<' => self.output.push_str("&lt;"),
                '>' => self.output.push_str("&gt;"),
                '"' => self.output.push_str("&quot;"),
                '\'' => self.output.push_str("&apos;"),
                _ => self.output.push(ch),
            }
        }
    }

    fn finish(mut self) -> String {
        // Close any remaining open tags
        while let Some(tag) = self.tag_stack.pop() {
            match tag {
                PangoTag::Bold => self.output.push_str("</b>"),
                PangoTag::Italic => self.output.push_str("</i>"),
                PangoTag::Strikethrough => self.output.push_str("</s>"),
                PangoTag::Link => self.output.push_str("</span>"),
            }
        }
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_and_italic() {
        let result = inline_to_pango("**bold** and *italic*");
        assert!(result.contains("<b>bold</b>"));
        assert!(result.contains("<i>italic</i>"));
    }

    #[test]
    fn code_span() {
        let result = inline_to_pango("use `println!`");
        assert!(result.contains("<span"));
        assert!(result.contains("println!"));
        assert!(result.contains("</span>"));
    }

    #[test]
    fn link() {
        let result = inline_to_pango("[click here](https://example.com)");
        assert!(result.contains("<span"));
        assert!(result.contains("click here"));
        assert!(result.contains("</span>"));
    }

    #[test]
    fn strikethrough() {
        let result = inline_to_pango("~~deleted~~");
        assert!(result.contains("<s>deleted</s>"));
    }

    #[test]
    fn escape_special_chars() {
        let result = inline_to_pango("a & b < c > d");
        assert!(result.contains("&amp;"));
        assert!(result.contains("&lt;"));
        assert!(result.contains("&gt;"));
    }

    #[test]
    fn nested_bold_italic() {
        let result = inline_to_pango("***both***");
        assert!(result.contains("<b>"));
        assert!(result.contains("<i>"));
    }
}
