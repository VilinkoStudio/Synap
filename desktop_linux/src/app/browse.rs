//! Browse mode — navigation, list management, search, pagination, tag filtering.

use relm4::prelude::*;

use super::App;
use super::message::AppMsg;
use crate::{
    domain::{ContentView, FocusMode},
    usecase::load_home,
};

impl App {
    pub(super) fn navigate(&mut self, view: ContentView, sender: &ComponentSender<Self>) {
        if self.state.content_view == view && self.state.focus_mode.is_browse() {
            return;
        }

        self.state.focus_mode = FocusMode::Browse;
        self.overlay_split_view.set_collapsed(false);
        self.state.content_view = view;
        self.state.sync_selection();
        self.rebuild_list(sender);
    }

    pub(super) fn clear_filters(&mut self, sender: &ComponentSender<Self>) {
        self.state.search_query.clear();
        self.state.home.tag_filter = crate::domain::TagFilter::default();
        self.state.content_view = ContentView::Notes;
        self.state.sync_selection();
        self.refresh_home(sender);
    }

    pub(super) fn rebuild_list(&self, _sender: &ComponentSender<Self>) {
        // Both Notes and Trash use FlowBox, synced via sync_home_feed.
        // This method is now a no-op — sync_ui handles all list rebuilding.
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
        };

        if let Some(cursor) = cursor {
            self.state.is_loading_more = true;
            self.sync_ui(sender);

            let core = self.core.clone();
            let sender = sender.clone();
            let is_trash = self.state.content_view == ContentView::Trash;
            let tag_filter = self.state.home.tag_filter.clone();
            gtk::glib::spawn_future_local(async move {
                let result = if is_trash {
                    crate::usecase::load_more_deleted_notes(core.as_ref(), &cursor)
                } else {
                    crate::usecase::load_more_notes(core.as_ref(), &cursor, &tag_filter)
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
                }
                self.rebuild_list(sender);
            }
            Err(error) => self.state.status = Some(format!("加载更多失败: {error}")),
        }
    }

    pub(super) fn toggle_tag_filter(&mut self, tag: String, sender: &ComponentSender<Self>) {
        let filter = &mut self.state.home.tag_filter;
        if !filter.tag_filter_enabled {
            // First toggle: enable filter, deselect this tag
            filter.tag_filter_enabled = true;
            filter.selected_tags = self.state.home.all_tags.clone();
            filter.selected_tags.retain(|t| t != &tag);
        } else if filter.selected_tags.contains(&tag) {
            filter.selected_tags.retain(|t| t != &tag);
        } else {
            filter.selected_tags.push(tag);
        }
        self.refresh_home(sender);
    }

    pub(super) fn toggle_untagged_filter(&mut self, sender: &ComponentSender<Self>) {
        let filter = &mut self.state.home.tag_filter;
        filter.tag_filter_enabled = true;
        filter.include_untagged = !filter.include_untagged;
        self.refresh_home(sender);
    }

    pub(super) fn toggle_all_tags(&mut self, sender: &ComponentSender<Self>) {
        self.state.home.tag_filter = crate::domain::TagFilter::default();
        self.refresh_home(sender);
    }

    pub(super) fn refresh_home(&mut self, sender: &ComponentSender<Self>) {
        let query = self.state.search_query.clone();
        let tag_filter = self.state.home.tag_filter.clone();
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = load_home(core.as_ref(), &query, &tag_filter);
            let _ = sender
                .input_sender()
                .send(AppMsg::HomeRefreshed(result));
        });
    }
}
