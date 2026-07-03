//! Focus mode — reading, editing, replying, delete/restore operations.

use adw::prelude::*;
use relm4::prelude::*;

use super::App;
use super::message::AppMsg;
use crate::{
    domain::{ContentView, FocusMode, WorkspaceMode},
    ui::util::{compact_single_line, parse_tags},
    usecase::load_home,
};

impl App {
    pub(super) fn enter_focus(&mut self, note_id: String, sender: &ComponentSender<Self>) {
        self.state.focus_mode = FocusMode::Reading(note_id.clone());
        self.state.selected_note_id = Some(note_id.clone());
        self.overlay_split_view.set_collapsed(true);
        self.content_stack.set_visible_child_name("reading");
        self.load_note_detail(note_id, sender);
    }

    pub(super) fn exit_focus(&mut self, sender: &ComponentSender<Self>) {
        self.state.focus_mode = FocusMode::Browse;
        self.state.selected_note_full = None;
        self.overlay_split_view.set_collapsed(false);
        self.reading_editor.borrow().set_read_only(true);

        let child_name = match self.state.content_view {
            ContentView::Notes | ContentView::Trash | ContentView::TagNotes => {
                if self.state.visible_notes().is_empty() {
                    "empty"
                } else {
                    "notes"
                }
            }
            ContentView::Tags => "tags",
            ContentView::Timeline => "timeline",
            ContentView::Settings => "settings",
        };
        self.content_stack.set_visible_child_name(child_name);
        self.rebuild_list(sender);
    }

    pub(super) fn start_create_note(&mut self) {
        self.state.focus_mode = FocusMode::Editing(WorkspaceMode::CreateDraft);
        self.state.draft_content.clear();
        self.state.draft_tags_text.clear();
        self.state.status = None;
        self.overlay_split_view.set_collapsed(true);
        self.editing_title_label.set_text("新建笔记");
        self.editing_hint_label
            .set_text("直接记录，不需要先分类。Markdown、清单和引用都可以原样输入。");
        self.reading_editor.borrow().set_read_only(false);
        self.reading_editor.borrow().set_content("");
    }

    pub(super) fn start_edit_note(&mut self) {
        let Some(detail) = self.state.selected_note_detail.clone() else {
            self.state.status = Some("请先选择一条笔记".to_string());
            return;
        };

        self.state.focus_mode = FocusMode::Editing(WorkspaceMode::EditDraft(detail.id));
        self.state.draft_content = detail.content.clone();
        self.state.draft_tags_text = detail.tags.join(", ");
        self.state.status = None;
        self.overlay_split_view.set_collapsed(true);
        self.editing_title_label.set_text("编辑笔记");
        self.editing_hint_label
            .set_text("保存后会更新这条笔记，并保留版本脉络。");
        self.reading_editor.borrow().set_read_only(false);
        self.reading_editor.borrow().set_content(&detail.content);
    }

    pub(super) fn start_reply_to_note(&mut self) {
        let Some(parent_id) = self.state.selected_note_id.clone() else {
            self.state.status = Some("请先选择一条要回复的笔记".to_string());
            return;
        };

        self.state.focus_mode = FocusMode::Editing(WorkspaceMode::ReplyDraft(parent_id));
        self.state.draft_content.clear();
        self.state.draft_tags_text.clear();
        self.state.status = None;
        self.overlay_split_view.set_collapsed(true);
        self.editing_title_label.set_text("回复笔记");
        let target = self
            .state
            .selected_note_detail
            .as_ref()
            .map(|d| compact_single_line(&d.content, 60))
            .unwrap_or_else(|| "当前笔记".to_string());
        self.editing_hint_label
            .set_text(&format!("回复目标：{target}"));
        self.reading_editor.borrow().set_read_only(false);
        self.reading_editor.borrow().set_content("");
    }

    pub(super) fn cancel_draft(&mut self, sender: &ComponentSender<Self>) {
        self.reading_editor.borrow().set_read_only(true);

        match &self.state.focus_mode {
            FocusMode::Editing(WorkspaceMode::ReplyDraft(_))
            | FocusMode::Editing(WorkspaceMode::EditDraft(_)) => {
                if let Some(note_id) = self.state.selected_note_id.clone() {
                    self.state.focus_mode = FocusMode::Reading(note_id.clone());
                    self.load_note_detail(note_id, sender);
                } else {
                    self.exit_focus(sender);
                }
            }
            _ => {
                self.exit_focus(sender);
            }
        }
        self.state.draft_content.clear();
        self.state.draft_tags_text.clear();
        self.state.status = None;
    }

    pub(super) fn save_draft(&mut self, sender: &ComponentSender<Self>) {
        let content = self.state.draft_content.trim().to_string();
        if content.is_empty() {
            self.state.status = Some("请输入笔记内容".to_string());
            return;
        }

        let tags = parse_tags(&self.state.draft_tags_text);
        let mode = match &self.state.focus_mode {
            FocusMode::Editing(m) => m.clone(),
            _ => return,
        };

        let result = match &mode {
            WorkspaceMode::CreateDraft => self.core.create_note(content, tags),
            WorkspaceMode::EditDraft(id) => self.core.edit_note(id, content, tags),
            WorkspaceMode::ReplyDraft(parent_id) => self.core.reply_note(parent_id, content, tags),
        };

        match result {
            Ok(note) => {
                self.state.draft_content.clear();
                self.state.draft_tags_text.clear();
                self.state.search_query.clear();
                self.state.status = None;
                self.reading_editor.borrow().set_read_only(true);

                if matches!(mode, WorkspaceMode::ReplyDraft(_)) {
                    self.toast_overlay.add_toast(adw::Toast::new("已发送回复"));
                    if let Some(parent_id) = self.state.selected_note_id.clone() {
                        self.state.focus_mode = FocusMode::Reading(parent_id.clone());
                        self.load_note_detail(parent_id, sender);
                    }
                } else {
                    let msg = if matches!(mode, WorkspaceMode::CreateDraft) {
                        "已创建笔记"
                    } else {
                        "已更新笔记"
                    };
                    self.toast_overlay.add_toast(adw::Toast::new(msg));
                    self.state.focus_mode = FocusMode::Reading(note.id.clone());
                    self.state.selected_note_id = Some(note.id.clone());
                    self.load_note_detail(note.id, sender);
                }

                let query = self.state.search_query.clone();
                if let Ok(home) = load_home(self.core.as_ref(), &query) {
                    self.state.home = home;
                    self.state.sync_selection();
                }
            }
            Err(error) => self.state.status = Some(format!("保存失败: {error}")),
        }
    }

    pub(super) fn confirm_delete_note(&self, sender: &ComponentSender<Self>) {
        if self.state.selected_note_id.is_none() {
            return;
        }

        let dialog = adw::MessageDialog::builder()
            .heading("删除笔记")
            .body("确定要删除这条笔记吗？删除后可在回收站中恢复。")
            .modal(true)
            .build();

        dialog.add_response("cancel", "取消");
        dialog.add_response("delete", "删除");
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let s = sender.input_sender().clone();
        dialog.connect_response(None, move |_, response| {
            if response == "delete" {
                let _ = s.send(AppMsg::ConfirmDeleteNote);
            }
        });

        dialog.present();
    }

    pub(super) fn delete_selected_note(&mut self, sender: &ComponentSender<Self>) {
        let Some(id) = self.state.selected_note_id.clone() else {
            return;
        };

        match self.core.delete_note(&id) {
            Ok(()) => {
                self.toast_overlay.add_toast(adw::Toast::new("已删除笔记"));
                self.exit_focus(sender);
                self.refresh_home(sender);
            }
            Err(error) => {
                self.state.status = Some(format!("删除失败: {error}"));
            }
        }
    }

    pub(super) fn restore_note(&mut self, id: &str, sender: &ComponentSender<Self>) {
        match self.core.restore_note(id) {
            Ok(()) => {
                self.toast_overlay.add_toast(adw::Toast::new("已恢复笔记"));
                self.refresh_home(sender);
            }
            Err(error) => {
                self.state.status = Some(format!("恢复失败: {error}"));
            }
        }
    }
}
