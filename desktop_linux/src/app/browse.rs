//! Browse mode — navigation, list management, search, pagination.

use adw::prelude::*;
use relm4::prelude::*;

use super::App;
use super::message::AppMsg;
use crate::{
    domain::{ContentView, FocusMode},
    ui::note_widgets::build_note_row,
    usecase::load_home,
};

impl App {
    pub(super) fn connect_note_list(&self, sender: &ComponentSender<Self>) {
        let sender_for_activate = sender.input_sender().clone();
        self.list_box.connect_row_activated(move |_, row| {
            let _ = sender_for_activate.send(AppMsg::NoteRowActivated(row.index() as u32));
        });
    }

    pub(super) fn navigate(&mut self, view: ContentView, sender: &ComponentSender<Self>) {
        if self.state.content_view == view && self.state.focus_mode.is_browse() {
            return;
        }

        self.state.focus_mode = FocusMode::Browse;
        self.overlay_split_view.set_collapsed(false);
        self.state.content_view = view;
        self.state.sync_selection();
        self.rebuild_list(sender);

        match view {
            ContentView::Tags => {
                let core = self.core.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let result = core.get_all_tags();
                    let _ = sender.input_sender().send(AppMsg::TagsLoaded(result));
                });
            }
            ContentView::Timeline => {
                let core = self.core.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let result = core.get_recent_sessions(None, Some(20));
                    let _ = sender
                        .input_sender()
                        .send(AppMsg::TimelineLoaded(result.map(|page| page.sessions)));
                });
            }
            ContentView::Settings => self.refresh_sync(sender),
            _ => {}
        }
    }

    pub(super) fn clear_filters(&mut self, sender: &ComponentSender<Self>) {
        self.state.selected_tag = None;
        self.state.search_query.clear();
        self.state.content_view = ContentView::Notes;
        self.state.tag_notes.clear();
        self.state.sync_selection();
        self.refresh_home(sender);
    }

    pub(super) fn rebuild_list(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let visible = self.state.visible_notes();
        let is_trash = self.state.content_view == ContentView::Trash;
        for note in &visible {
            self.list_box.append(&build_note_row(note, sender, is_trash));
        }

        if self.state.is_loading_more {
            let loading_row = gtk::ListBoxRow::new();
            let spinner = gtk::Spinner::new();
            spinner.set_margin_top(8);
            spinner.set_margin_bottom(8);
            spinner.start();
            loading_row.set_child(Some(&spinner));
            loading_row.set_activatable(false);
            self.list_box.append(&loading_row);
        }
    }

    pub(super) fn load_note_detail(&self, note_id: String, sender: &ComponentSender<Self>) {
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = crate::usecase::load_note_detail(core.as_ref(), &note_id);
            let _ = sender.input_sender().send(AppMsg::NoteDetailLoaded(result));
        });
    }

    pub(super) fn load_more_notes(&mut self, sender: &ComponentSender<Self>) {
        let cursor = match self.state.content_view {
            ContentView::Notes => self.state.home.notes_cursor.clone(),
            ContentView::Trash => self.state.home.deleted_notes_cursor.clone(),
            _ => None,
        };

        if let Some(cursor) = cursor {
            self.state.is_loading_more = true;
            self.sync_ui(sender);

            let core = self.core.clone();
            let sender = sender.clone();
            let is_trash = self.state.content_view == ContentView::Trash;
            gtk::glib::spawn_future_local(async move {
                let result = if is_trash {
                    crate::usecase::load_more_deleted_notes(core.as_ref(), &cursor)
                } else {
                    crate::usecase::load_more_notes(core.as_ref(), &cursor)
                };
                let _ = sender.input_sender().send(AppMsg::MoreNotesLoaded(result));
            });
        }
    }

    pub(super) fn finish_loading_more(
        &mut self,
        result: Result<
            (Vec<synap_core::dto::NoteDTO>, Option<String>, bool),
            synap_core::error::ServiceError,
        >,
        sender: &ComponentSender<Self>,
    ) {
        self.state.is_loading_more = false;
        match result {
            Ok((notes, next_cursor, has_more)) => {
                match self.state.content_view {
                    ContentView::Notes => {
                        self.state.home.notes.extend(notes);
                        self.state.home.notes_cursor = next_cursor;
                        self.state.home.has_more_notes = has_more;
                    }
                    ContentView::Trash => {
                        self.state.home.deleted_notes.extend(notes);
                        self.state.home.deleted_notes_cursor = next_cursor;
                        self.state.home.has_more_deleted_notes = has_more;
                    }
                    _ => {}
                }
                self.rebuild_list(sender);
            }
            Err(error) => self.state.status = Some(format!("加载更多失败: {error}")),
        }
    }

    pub(super) fn open_tag_notes(&mut self, tag: String, sender: &ComponentSender<Self>) {
        self.state.selected_tag = Some(tag.clone());
        self.state.content_view = ContentView::TagNotes;
        self.state.sync_selection();
        self.rebuild_list(sender);

        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.get_notes_by_tag(&tag, 50);
            let _ = sender.input_sender().send(AppMsg::TagNotesLoaded(result));
        });
    }

    pub(super) fn refresh_home(&mut self, sender: &ComponentSender<Self>) {
        let query = self.state.search_query.clone();
        match load_home(self.core.as_ref(), &query) {
            Ok(home) => {
                self.state.home = home;
                self.state.sync_selection();
                self.state.status = None;
            }
            Err(error) => self.state.status = Some(format!("加载失败: {error}")),
        }
        self.rebuild_list(sender);
    }
}
