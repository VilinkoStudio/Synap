//! WYSIWYG Markdown Editor — block-based rendering with inline formatting.
//!
//! Architecture:
//! - `model` — document model (MdBlock, BlockKind, ListItem)
//! - `parser` — pulldown-cmark → Vec<MdBlock>
//! - `markdown` — inline markdown → Pango markup
//! - `renderer` — MdBlock → GTK widget
//! - `widget` — WysiwygEditor: the main editor component with mode switching
//! - `css` — editor-specific CSS
//!
//! The editor has two modes:
//! - **Read-only**: renders markdown as styled widgets, no editing
//! - **Editable**: click a block → switches to raw markdown TextView; blur → re-render

pub mod css;
pub mod markdown;
pub mod model;
pub mod parser;
pub mod renderer;
pub mod widget;

pub use widget::WysiwygEditor;
