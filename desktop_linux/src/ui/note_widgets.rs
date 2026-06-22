use adw::prelude::*;
use relm4::prelude::*;
use synap_core::dto::NoteDTO;

use crate::{
    app::{App, message::AppMsg},
    domain::format_timestamp,
};

/// 笔记列表行（浏览模式）— 点击进入沉浸阅读
pub fn build_note_row(note: &NoteDTO, sender: &ComponentSender<App>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);

    let body = build_note_card_body(note);
    row.set_child(Some(&body));

    let note_id = note.id.clone();
    let s = sender.input_sender().clone();
    row.connect_activate(move |_| {
        let _ = s.send(AppMsg::OpenNoteFocus(note_id.clone()));
    });

    row
}

/// 可点击的笔记卡片（上下文面板中的关联笔记）— 点击进入沉浸阅读
pub fn build_clickable_note_row(
    note: &NoteDTO,
    sender: &ComponentSender<App>,
    note_id: String,
) -> gtk::Box {
    let card = build_note_card_body(note);
    card.add_css_class("card");

    let s = sender.input_sender().clone();
    let gesture = gtk::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| {
        let _ = s.send(AppMsg::OpenNoteFocus(note_id.clone()));
    });
    card.add_controller(gesture);

    card
}

fn build_note_card_body(note: &NoteDTO) -> gtk::Box {
    let preview = compact_preview(&note.content, 150);

    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.set_hexpand(true);
    card.set_margin_top(8);
    card.set_margin_bottom(8);
    card.set_margin_start(8);
    card.set_margin_end(8);
    card.add_css_class("synap-note-card");

    let content_label = gtk::Label::new(Some(&preview));
    content_label.add_css_class("synap-card-content");
    content_label.set_wrap(true);
    content_label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    content_label.set_justify(gtk::Justification::Left);
    content_label.set_halign(gtk::Align::Start);
    content_label.set_xalign(0.0);
    content_label.set_selectable(false);
    content_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content_label.set_lines(3);
    content_label.set_max_width_chars(88);
    card.append(&content_label);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_hexpand(true);
    footer.set_valign(gtk::Align::Center);

    let time_label = gtk::Label::new(Some(&format_timestamp(note.created_at)));
    time_label.add_css_class("caption");
    time_label.add_css_class("dim-label");
    time_label.add_css_class("synap-card-time");
    time_label.set_halign(gtk::Align::Start);
    time_label.set_xalign(0.0);
    footer.append(&time_label);

    let tags = build_tags_box(&note.tags, 6);
    footer.append(&tags);
    card.append(&footer);

    card
}

pub fn build_tags_box(tags: &[String], limit: usize) -> gtk::Box {
    let tags_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tags_box.set_halign(gtk::Align::Start);
    tags_box.set_hexpand(true);

    for tag in tags.iter().take(limit) {
        tags_box.append(&tag_chip(tag));
    }

    if tags.len() > limit {
        let more = gtk::Label::new(Some(&format!("+{}", tags.len() - limit)));
        more.add_css_class("caption");
        more.add_css_class("dim-label");
        tags_box.append(&more);
    }

    tags_box
}

pub fn tag_chip(tag: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(&format!("#{tag}")));
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label.add_css_class("synap-tag-chip");
    label.set_xalign(0.0);
    label
}

fn compact_preview(content: &str, max_chars: usize) -> String {
    let normalized = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(strip_markdown_markers)
        .collect::<Vec<_>>()
        .join("\n");

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

fn strip_markdown_markers(line: &str) -> String {
    let mut text = line.trim();

    while let Some(rest) = text.strip_prefix('#') {
        text = rest.trim_start();
    }

    for prefix in ["> ", "- [ ] ", "- [x] ", "- ", "* "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest;
            break;
        }
    }

    text.replace("**", "")
        .replace("***", "")
        .replace("~~", "")
        .replace("==", "")
        .replace("<u>", "")
        .replace("</u>", "")
}
