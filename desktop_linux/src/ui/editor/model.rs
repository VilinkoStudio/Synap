//! Document model — parsed markdown blocks with source byte ranges.

/// A block-level markdown element with its source span.
#[derive(Clone, Debug)]
pub struct MdBlock {
    pub kind: BlockKind,
    /// Byte offset of this block's start in the original markdown.
    pub source_start: usize,
    /// Byte offset past this block's end in the original markdown.
    pub source_end: usize,
}

/// The kind of a markdown block.
#[derive(Clone, Debug)]
pub enum BlockKind {
    Heading {
        level: u8,
        text: String, // raw inline markdown (without leading #)
    },
    Paragraph(String), // raw inline markdown
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Blockquote(String), // inner markdown (may contain nested blocks)
    BulletList(Vec<ListItem>),
    OrderedList(Vec<ListItem>),
    HorizontalRule,
    Blank,
}

/// A single list item.
#[derive(Clone, Debug)]
pub struct ListItem {
    pub text: String,    // raw inline markdown
    pub checked: Option<bool>, // for task lists: Some(true/false), None for plain items
}

impl BlockKind {
    /// Returns the raw markdown text for this block (for round-trip editing).
    pub fn to_markdown(&self) -> String {
        match self {
            BlockKind::Heading { level, text } => {
                format!("{} {}", "#".repeat(*level as usize), text)
            }
            BlockKind::Paragraph(text) => text.clone(),
            BlockKind::CodeBlock { language, code } => {
                let lang = language.as_deref().unwrap_or("");
                format!("```{}\n{}\n```", lang, code)
            }
            BlockKind::Blockquote(text) => {
                text.lines()
                    .map(|l| format!("> {}", l))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockKind::BulletList(items) => {
                items
                    .iter()
                    .map(|item| {
                        let prefix = match item.checked {
                            Some(true) => "- [x] ",
                            Some(false) => "- [ ] ",
                            None => "- ",
                        };
                        format!("{}{}", prefix, item.text)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockKind::OrderedList(items) => {
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| format!("{}. {}", i + 1, item.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockKind::HorizontalRule => "---".to_string(),
            BlockKind::Blank => String::new(),
        }
    }
}
