//! Block → GTK widget renderer.
//!
//! Converts each MdBlock into a display-ready GTK widget using Pango markup
//! for inline formatting.

use gtk::prelude::*;

use super::markdown::inline_to_pango;
use super::model::{BlockKind, MdBlock};

/// Render a list of blocks into a vertical box of widgets.
pub fn render_blocks(blocks: &[MdBlock], container: &gtk::Box) {
    // Remove existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    for block in blocks {
        let widget = render_block(block);
        container.append(&widget);
    }
}

/// Render a single block into a display widget.
pub fn render_block(block: &MdBlock) -> gtk::Widget {
    match &block.kind {
        BlockKind::Heading { level, text } => render_heading(*level, text),
        BlockKind::Paragraph(text) => render_paragraph(text),
        BlockKind::CodeBlock { language, code } => render_code_block(language.as_deref(), code),
        BlockKind::Blockquote(text) => render_blockquote(text),
        BlockKind::BulletList(items) => render_bullet_list(items),
        BlockKind::OrderedList(items) => render_ordered_list(items),
        BlockKind::HorizontalRule => render_hr(),
        BlockKind::Blank => render_blank(),
    }
}

fn render_heading(level: u8, text: &str) -> gtk::Widget {
    let pango = inline_to_pango(text);
    let label = gtk::Label::new(None);
    label.set_markup(&pango);
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_selectable(true);
    label.add_css_class("synap-editor-heading");
    label.add_css_class(&format!("synap-editor-heading-{}", level));
    label.upcast()
}

fn render_paragraph(text: &str) -> gtk::Widget {
    let pango = inline_to_pango(text);
    let label = gtk::Label::new(None);
    label.set_markup(&pango);
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_selectable(true);
    label.add_css_class("synap-editor-paragraph");
    label.upcast()
}

fn render_code_block(language: Option<&str>, code: &str) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.add_css_class("synap-editor-code-block");

    // Language label
    if let Some(lang) = language {
        if !lang.is_empty() {
            let lang_label = gtk::Label::new(Some(lang));
            lang_label.set_xalign(0.0);
            lang_label.add_css_class("synap-editor-code-lang");
            outer.append(&lang_label);
        }
    }

    // Code content — monospace, no wrapping, selectable
    let escaped = escape_pango(code);
    let code_label = gtk::Label::new(None);
    code_label.set_markup(&format!(
        "<span font_family=\"monospace\" font_size=\"smaller\">{}</span>",
        escaped
    ));
    code_label.set_xalign(0.0);
    code_label.set_wrap(false);
    code_label.set_selectable(true);
    code_label.add_css_class("synap-editor-code-content");
    outer.append(&code_label);

    outer.upcast()
}

fn render_blockquote(text: &str) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    outer.add_css_class("synap-editor-blockquote");

    // Left accent bar
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar.add_css_class("synap-editor-blockquote-bar");
    bar.set_width_request(3);
    outer.append(&bar);

    // Content — render each line with inline formatting
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    content_box.set_hexpand(true);
    content_box.add_css_class("synap-editor-blockquote-content");

    for line in text.lines() {
        let trimmed = line.trim_start_matches("> ");
        let pango = inline_to_pango(trimmed);
        let label = gtk::Label::new(None);
        label.set_markup(&pango);
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_selectable(true);
        content_box.append(&label);
    }

    outer.append(&content_box);
    outer.upcast()
}

fn render_bullet_list(items: &[super::model::ListItem]) -> gtk::Widget {
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    list_box.add_css_class("synap-editor-list");

    for item in items {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        // Bullet or checkbox
        let marker = match item.checked {
            Some(true) => "☑",
            Some(false) => "☐",
            None => "•",
        };
        let marker_label = gtk::Label::new(Some(marker));
        marker_label.set_xalign(0.5);
        marker_label.add_css_class("synap-editor-list-marker");
        row.append(&marker_label);

        // Content
        let pango = inline_to_pango(&item.text);
        let content_label = gtk::Label::new(None);
        content_label.set_markup(&pango);
        content_label.set_xalign(0.0);
        content_label.set_wrap(true);
        content_label.set_selectable(true);
        content_label.set_hexpand(true);
        row.append(&content_label);

        list_box.append(&row);
    }

    list_box.upcast()
}

fn render_ordered_list(items: &[super::model::ListItem]) -> gtk::Widget {
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    list_box.add_css_class("synap-editor-list");

    for (i, item) in items.iter().enumerate() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        // Number
        let num_label = gtk::Label::new(Some(&format!("{}.", i + 1)));
        num_label.set_xalign(1.0);
        num_label.set_width_request(24);
        num_label.add_css_class("synap-editor-list-marker");
        row.append(&num_label);

        // Content
        let pango = inline_to_pango(&item.text);
        let content_label = gtk::Label::new(None);
        content_label.set_markup(&pango);
        content_label.set_xalign(0.0);
        content_label.set_wrap(true);
        content_label.set_selectable(true);
        content_label.set_hexpand(true);
        row.append(&content_label);

        list_box.append(&row);
    }

    list_box.upcast()
}

fn render_hr() -> gtk::Widget {
    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("synap-editor-hr");
    separator.upcast()
}

fn render_blank() -> gtk::Widget {
    let label = gtk::Label::new(Some(" "));
    label.upcast()
}

/// Escape text for Pango markup.
fn escape_pango(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
