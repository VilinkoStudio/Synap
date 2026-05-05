use adw::prelude::*;
use relm4::prelude::*;

use crate::{
    app::{App, message::AppMsg},
    domain::NoteDetailViewModel,
};

pub fn present_note_editor(
    sender: &ComponentSender<App>,
    note_id: Option<String>,
    is_reply: bool,
    selected_detail: Option<NoteDetailViewModel>,
) {
    let (title, initial_content, initial_tags) = if is_reply {
        ("回复笔记", String::new(), Vec::new())
    } else if note_id.is_some() {
        let Some(detail) = selected_detail else {
            return;
        };
        ("编辑笔记", detail.content, detail.tags)
    } else {
        ("新建笔记", String::new(), Vec::new())
    };

    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(720)
        .content_height(560)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();

    let cancel_button = gtk::Button::with_label("取消");
    let save_button = gtk::Button::with_label("保存");
    save_button.add_css_class("suggested-action");

    header.pack_start(&cancel_button);
    header.pack_end(&save_button);

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 16);
    shell.set_margin_top(24);
    shell.set_margin_bottom(24);
    shell.set_margin_start(24);
    shell.set_margin_end(24);

    let content_buffer = gtk::TextBuffer::new(None);
    content_buffer.set_text(&initial_content);
    let content_view = gtk::TextView::with_buffer(&content_buffer);
    content_view.set_vexpand(true);
    content_view.set_wrap_mode(gtk::WrapMode::WordChar);
    content_view.set_top_margin(8);
    content_view.set_bottom_margin(8);
    content_view.set_left_margin(8);
    content_view.set_right_margin(8);

    let content_scroller = gtk::ScrolledWindow::new();
    content_scroller.set_vexpand(true);
    content_scroller.set_min_content_height(320);
    content_scroller.set_child(Some(&content_view));

    let tags_entry = gtk::Entry::new();
    tags_entry.set_placeholder_text(Some("标签，例如 rust, idea, diary"));
    tags_entry.set_text(&initial_tags.join(", "));

    let status_label = gtk::Label::new(None);
    status_label.add_css_class("error");
    status_label.set_visible(false);

    shell.append(&content_scroller);
    shell.append(&tags_entry);
    shell.append(&status_label);

    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&shell));

    dialog.set_child(Some(&toolbar_view));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_for_cancel.close();
    });

    let dialog_for_save = dialog.clone();
    let input_sender = sender.input_sender().clone();
    let note_id_for_save = note_id.clone();
    save_button.connect_clicked(move |_| {
        let (start, end) = content_buffer.bounds();
        let text = content_buffer.text(&start, &end, false).to_string();
        let trimmed = text.trim().to_string();

        if trimmed.is_empty() {
            status_label.set_text("请输入笔记内容");
            status_label.set_visible(true);
            return;
        }

        let tags = parse_tags(&tags_entry.text());
        if is_reply {
            if let Some(parent_id) = &note_id_for_save {
                let _ = input_sender.send(AppMsg::SaveReply {
                    parent_id: parent_id.clone(),
                    content: trimmed,
                    tags,
                });
            }
        } else {
            let _ = input_sender.send(AppMsg::SaveNote {
                id: note_id_for_save.clone(),
                content: trimmed,
                tags,
            });
        }

        dialog_for_save.close();
    });

    dialog.present(None::<&gtk::Widget>);
}

fn parse_tags(raw: &str) -> Vec<String> {
    let mut tags = Vec::new();

    for tag in raw.split([',', '，']) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        if tags.iter().any(|existing: &String| existing == trimmed) {
            continue;
        }
        tags.push(trimmed.to_string());
    }

    tags
}
