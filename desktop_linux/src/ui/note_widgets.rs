use adw::prelude::*;
use relm4::prelude::*;
use synap_core::dto::NoteDTO;

use crate::{
    app::{App, message::AppMsg},
    domain::{NoteListItemViewModel, format_timestamp},
};

pub fn build_note_row(note: &NoteDTO) -> gtk::ListBoxRow {
    let preview = NoteListItemViewModel::from(note).preview;

    let action_row = adw::ActionRow::new();
    action_row.set_title(&preview);
    action_row.set_subtitle(&format_tags(&note.tags));
    action_row.set_activatable(true);

    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&action_row));
    row
}

pub fn build_clickable_note_row(
    note: &NoteDTO,
    sender: &ComponentSender<App>,
    note_id: String,
) -> adw::ActionRow {
    let preview = NoteListItemViewModel::from(note).preview;

    let action_row = adw::ActionRow::new();
    action_row.set_title(&preview);
    action_row.set_subtitle(&format_tags(&note.tags));
    action_row.set_activatable(true);

    let gesture = gtk::GestureClick::new();
    let sender_clone = sender.input_sender().clone();
    gesture.connect_released(move |_, _, _, _| {
        let _ = sender_clone.send(AppMsg::OpenNoteDetail(note_id.clone()));
    });
    action_row.add_controller(gesture);

    action_row
}

pub fn build_waterfall_card(note: &NoteDTO, sender: &ComponentSender<App>) -> gtk::FlowBoxChild {
    let preview = NoteListItemViewModel::from(note).preview;

    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.set_margin_top(12);
    card.set_margin_bottom(12);
    card.set_margin_start(12);
    card.set_margin_end(12);
    card.set_hexpand(true);

    let content_label = gtk::Label::new(Some(&preview));
    content_label.set_wrap(true);
    content_label.set_max_width_chars(15);
    content_label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    content_label.set_justify(gtk::Justification::Left);
    content_label.set_halign(gtk::Align::Start);
    content_label.set_xalign(0.0);
    content_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    card.append(&content_label);

    if !note.tags.is_empty() {
        let tags_label = gtk::Label::new(Some(&format_tags(&note.tags)));
        tags_label.add_css_class("caption");
        tags_label.set_halign(gtk::Align::Start);
        tags_label.set_xalign(0.0);
        tags_label.set_wrap(true);
        tags_label.set_max_width_chars(15);
        card.append(&tags_label);
    }

    let time_label = gtk::Label::new(Some(&format_timestamp(note.created_at)));
    time_label.add_css_class("caption");
    time_label.set_halign(gtk::Align::Start);
    time_label.set_xalign(0.0);
    card.append(&time_label);

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

fn format_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<_>>()
        .join("  ")
}
