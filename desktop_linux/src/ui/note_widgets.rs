use adw::prelude::*;
use relm4::prelude::*;
use synap_core::dto::NoteDTO;

use crate::{
    app::{App, message::AppMsg},
    domain::{NoteListItemViewModel, format_timestamp},
};

pub fn build_note_row(note: &NoteDTO) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    row.set_child(Some(&build_note_card_body(note, NoteCardMode::List)));
    row
}

pub fn build_clickable_note_row(
    note: &NoteDTO,
    sender: &ComponentSender<App>,
    note_id: String,
) -> gtk::Box {
    let card = build_note_card_body(note, NoteCardMode::Related);
    card.add_css_class("card");

    let gesture = gtk::GestureClick::new();
    let sender_clone = sender.input_sender().clone();
    gesture.connect_released(move |_, _, _, _| {
        let _ = sender_clone.send(AppMsg::OpenNoteDetail(note_id.clone()));
    });
    card.add_controller(gesture);

    card
}

pub fn build_waterfall_card(note: &NoteDTO, sender: &ComponentSender<App>) -> gtk::FlowBoxChild {
    let card = build_note_card_body(note, NoteCardMode::Waterfall);
    card.add_css_class("card");

    let gesture = gtk::GestureClick::new();
    let note_id = note.id.clone();
    let sender_clone = sender.input_sender().clone();
    gesture.connect_released(move |_, _, _, _| {
        let _ = sender_clone.send(AppMsg::OpenNoteDetail(note_id.clone()));
    });
    card.add_controller(gesture);

    let flow_child = gtk::FlowBoxChild::new();
    flow_child.set_child(Some(&card));
    flow_child
}

#[derive(Clone, Copy)]
enum NoteCardMode {
    List,
    Waterfall,
    Related,
}

fn build_note_card_body(note: &NoteDTO, mode: NoteCardMode) -> gtk::Box {
    let preview = match mode {
        NoteCardMode::List => compact_preview(&note.content, 150),
        NoteCardMode::Waterfall => NoteListItemViewModel::from(note).preview,
        NoteCardMode::Related => compact_preview(&note.content, 260),
    };

    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.set_hexpand(true);
    card.set_margin_top(12);
    card.set_margin_bottom(12);
    card.set_margin_start(14);
    card.set_margin_end(14);

    let content_label = gtk::Label::new(Some(&preview));
    content_label.set_wrap(true);
    content_label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    content_label.set_justify(gtk::Justification::Left);
    content_label.set_halign(gtk::Align::Start);
    content_label.set_xalign(0.0);
    content_label.set_selectable(false);
    content_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    match mode {
        NoteCardMode::List => {
            content_label.set_lines(3);
            content_label.set_max_width_chars(88);
        }
        NoteCardMode::Waterfall => {
            content_label.set_lines(8);
            content_label.set_max_width_chars(42);
        }
        NoteCardMode::Related => {
            content_label.set_lines(6);
            content_label.set_max_width_chars(72);
        }
    }
    card.append(&content_label);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    footer.set_hexpand(true);
    footer.set_valign(gtk::Align::Center);

    let time_label = gtk::Label::new(Some(&format_timestamp(note.created_at)));
    time_label.add_css_class("caption");
    time_label.add_css_class("dim-label");
    time_label.set_halign(gtk::Align::Start);
    time_label.set_xalign(0.0);
    footer.append(&time_label);

    let tags = build_tags_box(
        &note.tags,
        match mode {
            NoteCardMode::Waterfall => 4,
            NoteCardMode::Related => 5,
            NoteCardMode::List => 6,
        },
    );
    footer.append(&tags);
    card.append(&footer);

    card
}

pub fn build_tags_box(tags: &[String], limit: usize) -> gtk::Box {
    let tags_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
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
