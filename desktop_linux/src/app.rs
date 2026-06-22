pub mod message;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use relm4::prelude::*;

#[allow(unused_imports)]
use crate::{
    app::message::AppMsg,
    core::DesktopCore,
    domain::{AppState, ContentView, FocusMode, NoteLayout, Theme, WorkspaceMode},
    ui::{
        editor::WysiwygEditor,
        note_widgets::{build_clickable_note_row, build_note_row, tag_chip},
        shell::build_content_pages,
        theme::apply_theme,
    },
    usecase::load_home,
};

pub struct App {
    core: Rc<dyn DesktopCore>,
    state: AppState,
    toast_overlay: adw::ToastOverlay,
    overlay_split_view: adw::OverlaySplitView,
    content_stack: gtk::Stack,
    list_box: gtk::ListBox,
    empty_page: adw::StatusPage,

    // reading
    reading_context_panel: gtk::ScrolledWindow,
    reading_editor: RefCell<WysiwygEditor>,
    reading_tags_box: gtk::Box,
    reading_meta_label: gtk::Label,
    reading_origins_box: gtk::Box,
    reading_replies_box: gtk::Box,
    reading_versions_box: gtk::Box,

    // editing (shares reading_editor, toggles read_only)
    editing_title_label: gtk::Label,
    editing_hint_label: gtk::Label,
    editing_tags_entry: gtk::Entry,

    // settings
    theme_dropdown: gtk::DropDown,
    sync_listener_row: adw::ActionRow,
    sync_addresses_row: adw::ActionRow,
    sync_identity_row: adw::ActionRow,
    sync_signing_row: adw::ActionRow,
    sync_error_label: gtk::Label,
    sync_host_entry: gtk::Entry,
    sync_port_entry: gtk::Entry,
    sync_discovered_box: gtk::Box,
    sync_connections_box: gtk::Box,
    sync_peers_box: gtk::Box,
    sync_sessions_box: gtk::Box,

    // tags
    tags_flow_box: gtk::FlowBox,

    // timeline
    timeline_container: gtk::Box,
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = Rc<dyn DesktopCore>;
    type Input = AppMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_title: Some("Synap"),
            set_default_size: (900, 640),

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                #[local_ref]
                overlay_split_view -> adw::OverlaySplitView {
                    set_sidebar_width_fraction: 0.24,
                    set_min_sidebar_width: 200.0,
                    set_max_sidebar_width: 300.0,

                    #[wrap(Some)]
                    set_sidebar = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "synap-sidebar",

                        gtk::Label {
                            set_label: "Synap",
                            add_css_class: "title-2",
                            set_xalign: 0.0,
                            set_margin_top: 16,
                            set_margin_start: 16,
                            set_margin_bottom: 8,
                        },

                        gtk::ListBox {
                            add_css_class: "navigation-sidebar",
                            set_selection_mode: gtk::SelectionMode::Single,
                            set_margin_top: 4,
                            set_margin_start: 6,
                            set_margin_end: 6,
                            connect_row_selected[sender] => move |_, row| {
                                let Some(row) = row else { return };
                                let view = match row.index() {
                                    0 => ContentView::Notes,
                                    1 => ContentView::Trash,
                                    2 => ContentView::Tags,
                                    3 => ContentView::Timeline,
                                    _ => return,
                                };
                                sender.input(AppMsg::Navigate(view));
                            },

                            gtk::ListBoxRow {
                                #[watch]
                                set_css_classes: if model.state.content_view == ContentView::Notes {
                                    &["synap-nav-row", "active"]
                                } else {
                                    &["synap-nav-row"]
                                },
                                set_activatable: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 10,
                                    set_hexpand: true,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    gtk::Image { set_icon_name: Some("document-open-symbolic") },
                                    gtk::Label { set_label: "笔记列表", set_xalign: 0.0, set_hexpand: true }
                                }
                            },

                            gtk::ListBoxRow {
                                #[watch]
                                set_css_classes: if model.state.content_view == ContentView::Trash {
                                    &["synap-nav-row", "active"]
                                } else {
                                    &["synap-nav-row"]
                                },
                                set_activatable: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 10,
                                    set_hexpand: true,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    gtk::Image { set_icon_name: Some("user-trash-symbolic") },
                                    gtk::Label { set_label: "回收站", set_xalign: 0.0, set_hexpand: true }
                                }
                            },

                            gtk::ListBoxRow {
                                #[watch]
                                set_css_classes: if matches!(model.state.content_view, ContentView::Tags | ContentView::TagNotes) {
                                    &["synap-nav-row", "active"]
                                } else {
                                    &["synap-nav-row"]
                                },
                                set_activatable: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 10,
                                    set_hexpand: true,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    gtk::Image { set_icon_name: Some("tag-symbolic") },
                                    gtk::Label { set_label: "标签", set_xalign: 0.0, set_hexpand: true }
                                }
                            },

                            gtk::ListBoxRow {
                                #[watch]
                                set_css_classes: if model.state.content_view == ContentView::Timeline {
                                    &["synap-nav-row", "active"]
                                } else {
                                    &["synap-nav-row"]
                                },
                                set_activatable: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 10,
                                    set_hexpand: true,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    gtk::Image { set_icon_name: Some("view-list-symbolic") },
                                    gtk::Label { set_label: "时间线", set_xalign: 0.0, set_hexpand: true }
                                }
                            }
                        },

                        gtk::Box { set_vexpand: true },

                        gtk::ListBox {
                            add_css_class: "navigation-sidebar",
                            set_selection_mode: gtk::SelectionMode::Single,
                            set_margin_bottom: 4,
                            set_margin_start: 6,
                            set_margin_end: 6,
                            connect_row_selected[sender] => move |_, row| {
                                if row.is_some() {
                                    sender.input(AppMsg::Navigate(ContentView::Settings));
                                }
                            },

                            gtk::ListBoxRow {
                                #[watch]
                                set_css_classes: if model.state.content_view == ContentView::Settings {
                                    &["synap-nav-row", "active"]
                                } else {
                                    &["synap-nav-row"]
                                },
                                set_activatable: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 10,
                                    set_hexpand: true,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    gtk::Image { set_icon_name: Some("preferences-system-symbolic") },
                                    gtk::Label { set_label: "设置", set_xalign: 0.0, set_hexpand: true }
                                }
                            }
                        }
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        adw::HeaderBar {
                            set_show_end_title_buttons: true,

                            // ── Start: add (browse) / back (reading/editing) ──
                            pack_start = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 150,
                                #[watch]
                                set_visible_child_name: match model.state.focus_mode {
                                    FocusMode::Browse => "browse",
                                    _ => "other",
                                },

                                add_named[Some("browse")] = &gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    set_tooltip_text: Some("新建笔记"),
                                    add_css_class: "flat",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::StartCreateNote);
                                    }
                                },

                                add_named[Some("other")] = &gtk::Button {
                                    set_icon_name: "go-previous-symbolic",
                                    set_tooltip_text: Some("返回"),
                                    add_css_class: "flat",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::ExitFocus);
                                    }
                                },
                            },

                            // ── Title: switches between modes ──
                            #[wrap(Some)]
                            set_title_widget = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 150,
                                #[watch]
                                set_visible_child_name: match model.state.focus_mode {
                                    FocusMode::Browse => "browse",
                                    FocusMode::Reading(_) => "reading",
                                    FocusMode::Editing(_) => "editing",
                                },

                                add_named[Some("browse")] = &adw::Clamp {
                                    set_maximum_size: 420,
                                    gtk::SearchEntry {
                                        set_placeholder_text: Some("搜索内容或标签"),
                                        set_hexpand: true,
                                        #[watch]
                                        set_visible: model.state.content_view != ContentView::Settings,
                                        connect_search_changed[sender] => move |entry| {
                                            sender.input(AppMsg::SearchChanged(entry.text().to_string()));
                                        }
                                    }
                                },

                                add_named[Some("reading")] = &gtk::Label {
                                    add_css_class: "caption",
                                    add_css_class: "dim-label",
                                    #[watch]
                                    set_label: model.reading_meta().as_str(),
                                },

                                add_named[Some("editing")] = &gtk::Label {
                                    add_css_class: "title",
                                    #[watch]
                                    set_text: model.editing_title_label.text().as_str(),
                                },
                            },

                            // ── End: action buttons per mode ──
                            pack_end = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 150,
                                #[watch]
                                set_visible_child_name: match model.state.focus_mode {
                                    FocusMode::Browse => "browse",
                                    FocusMode::Reading(_) => "reading",
                                    FocusMode::Editing(_) => "editing",
                                },

                                add_named[Some("browse")] = &gtk::Button {
                                    set_icon_name: "edit-clear-symbolic",
                                    set_tooltip_text: Some("清除筛选"),
                                    add_css_class: "flat",
                                    #[watch]
                                    set_visible: model.state.content_view == ContentView::TagNotes || !model.state.search_query.is_empty(),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::ClearFilters);
                                    }
                                },

                                add_named[Some("reading")] = &gtk::Box {
                                    set_spacing: 4,

                                    gtk::ToggleButton {
                                        set_icon_name: "view-sidebar-end-symbolic",
                                        set_tooltip_text: Some("上下文面板"),
                                        add_css_class: "flat",
                                        #[watch]
                                        set_active: model.state.context_panel_open,
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::ToggleContextPanel);
                                        }
                                    },
                                    gtk::Button {
                                        set_icon_name: "mail-reply-sender-symbolic",
                                        set_tooltip_text: Some("回复笔记"),
                                        add_css_class: "flat",
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::ReplyToNote);
                                        }
                                    },
                                    gtk::Button {
                                        set_icon_name: "document-edit-symbolic",
                                        set_tooltip_text: Some("编辑笔记"),
                                        add_css_class: "flat",
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::EditNote);
                                        }
                                    },
                                    gtk::Button {
                                        set_icon_name: "user-trash-symbolic",
                                        set_tooltip_text: Some("删除笔记"),
                                        add_css_class: "flat",
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::DeleteNote);
                                        }
                                    },
                                },

                                add_named[Some("editing")] = &gtk::Box {
                                    set_spacing: 8,

                                    gtk::Button {
                                        set_label: "取消",
                                        add_css_class: "flat",
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::CancelDraft);
                                        }
                                    },
                                    gtk::Button {
                                        set_label: "保存",
                                        add_css_class: "suggested-action",
                                        connect_clicked[sender] => move |_| {
                                            sender.input(AppMsg::SaveDraft);
                                        }
                                    },
                                },
                            },
                        },

                        gtk::Label {
                            #[watch]
                            set_visible: model.state.status.is_some(),
                            #[watch]
                            set_text: model.state.status.as_deref().unwrap_or(""),
                            add_css_class: "error",
                            set_margin_start: 18,
                            set_margin_end: 18,
                        },

                        #[local_ref]
                        content_stack -> gtk::Stack {}
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let core = init;
        let mut state = AppState::default();

        match load_home(core.as_ref(), "") {
            Ok(home) => {
                state.home = home;
                state.sync_selection();
            }
            Err(error) => {
                state.status = Some(format!("初始化失败: {error}"));
            }
        }

        apply_theme(state.theme);
        let toast_overlay = adw::ToastOverlay::new();
        let overlay_split_view = adw::OverlaySplitView::new();
        let pages = build_content_pages(&state, &sender);
        let content_stack = pages.content_stack.clone();

        let model = App {
            core: core.clone(),
            state,
            toast_overlay: toast_overlay.clone(),
            overlay_split_view: overlay_split_view.clone(),
            list_box: pages.list_box.clone(),
            content_stack: pages.content_stack,
            empty_page: pages.empty_page,

            reading_context_panel: pages.reading_context_panel,
            reading_editor: RefCell::new(pages.reading_editor),
            reading_tags_box: pages.reading_tags_box,
            reading_meta_label: pages.reading_meta_label,
            reading_origins_box: pages.reading_origins_box,
            reading_replies_box: pages.reading_replies_box,
            reading_versions_box: pages.reading_versions_box,

            editing_title_label: pages.editing_title_label,
            editing_hint_label: pages.editing_hint_label,
            editing_tags_entry: pages.editing_tags_entry,

            theme_dropdown: pages.theme_dropdown,
            sync_listener_row: pages.sync_listener_row,
            sync_addresses_row: pages.sync_addresses_row,
            sync_identity_row: pages.sync_identity_row,
            sync_signing_row: pages.sync_signing_row,
            sync_error_label: pages.sync_error_label,
            sync_host_entry: pages.sync_host_entry,
            sync_port_entry: pages.sync_port_entry,
            sync_discovered_box: pages.sync_discovered_box,
            sync_connections_box: pages.sync_connections_box,
            sync_peers_box: pages.sync_peers_box,
            sync_sessions_box: pages.sync_sessions_box,
            tags_flow_box: pages.tags_flow_box,
            timeline_container: pages.timeline_container,
        };

        let widgets = view_output!();
        model.connect_note_list(&sender);
        model.rebuild_list(&sender);
        model.sync_ui(&sender);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            // ── Browse navigation ──
            AppMsg::Navigate(view) => self.navigate(view, &sender),
            AppMsg::SearchChanged(query) => {
                self.state.search_query = query;
                self.refresh_home(&sender);
            }
            AppMsg::LayoutChanged(layout) => {
                self.state.layout = layout;
                self.rebuild_list(&sender);
            }
            AppMsg::ClearFilters => self.clear_filters(&sender),

            // ── Focus mode ──
            AppMsg::OpenNoteFocus(note_id) => self.enter_focus(note_id, &sender),
            AppMsg::NoteRowActivated(index) => {
                let visible = self.state.visible_notes();
                if let Some(note) = visible.get(index as usize) {
                    let note_id = note.id.clone();
                    self.enter_focus(note_id, &sender);
                }
            }
            AppMsg::ExitFocus => self.exit_focus(&sender),
            AppMsg::NoteDetailLoaded(result) => match result {
                Ok(data) => {
                    self.state.selected_note_id = Some(data.note.id.clone());
                    self.state.selected_note_detail = Some(data.to_view_model());
                    self.state.selected_note_full = Some(data);
                    self.state.status = None;
                }
                Err(error) => {
                    self.state.status = Some(format!("加载详情失败: {error}"));
                }
            },

            // ── Context panel ──
            AppMsg::ToggleContextPanel => {
                self.state.context_panel_open = !self.state.context_panel_open;
            }

            // ── Editing ──
            AppMsg::StartCreateNote => self.start_create_note(),
            AppMsg::StartEditNote => self.start_edit_note(),
            AppMsg::StartReplyToNote => self.start_reply_to_note(),
            AppMsg::DraftContentChanged(value) => self.state.draft_content = value,
            AppMsg::DraftTagsChanged(value) => self.state.draft_tags_text = value,
            AppMsg::SaveDraft => self.save_draft(&sender),
            AppMsg::CancelDraft => self.cancel_draft(&sender),

            // ── Note operations (from reading toolbar) ──
            AppMsg::EditNote => self.start_edit_note(),
            AppMsg::ReplyToNote => self.start_reply_to_note(),
            AppMsg::DeleteNote => self.delete_selected_note(&sender),

            // ── Theme ──
            AppMsg::ThemeChanged(theme) => {
                self.state.theme = theme;
                apply_theme(theme);
            }

            // ── List loading ──
            AppMsg::LoadMoreNotes => self.load_more_notes(&sender),
            AppMsg::MoreNotesLoaded(result) => self.finish_loading_more(result, &sender),

            // ── Tags ──
            AppMsg::TagSelected(tag) => self.open_tag_notes(tag, &sender),
            AppMsg::TagsLoaded(result) => match result {
                Ok(tags) => {
                    self.state.all_tags = tags;
                    self.state.status = None;
                }
                Err(error) => self.state.status = Some(format!("加载标签失败: {error}")),
            },
            AppMsg::TagNotesLoaded(result) => match result {
                Ok(notes) => {
                    self.state.tag_notes = notes;
                    self.state.sync_selection();
                    self.rebuild_list(&sender);
                    self.state.status = None;
                }
                Err(error) => self.state.status = Some(format!("加载标签笔记失败: {error}")),
            },

            // ── Timeline ──
            AppMsg::TimelineLoaded(result) => match result {
                Ok(sessions) => {
                    self.state.timeline_sessions = sessions;
                    self.state.status = None;
                }
                Err(error) => self.state.status = Some(format!("加载时间线失败: {error}")),
            },

            // ── Sync ──
            AppMsg::RefreshSync => self.refresh_sync(&sender),
            AppMsg::SyncOverviewLoaded {
                listener,
                identity,
                peers,
                sessions,
                discovered_peers,
                connections,
            } => self.finish_refresh_sync(
                listener,
                identity,
                peers,
                sessions,
                discovered_peers,
                connections,
            ),
            AppMsg::UpdateSyncHost(value) => self.state.sync.host_input = value,
            AppMsg::UpdateSyncPort(value) => self.state.sync.port_input = value,
            AppMsg::AddSyncConnection => self.add_sync_connection(),
            AppMsg::DeleteSyncConnection(id) => self.delete_sync_connection(&id),
            AppMsg::PairSyncConnection(id) => {
                if let Some(conn) = self
                    .state
                    .sync
                    .connections
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                {
                    self.start_sync_pair(conn.host, conn.port, &sender);
                }
            }
            AppMsg::PairDiscoveredPeer { host, port } => self.start_sync_pair(host, port, &sender),
            AppMsg::TrustPeer { public_key, note } => self.trust_peer(public_key, note),
            AppMsg::UpdatePeerNote { peer_id, note } => self.update_peer_note(peer_id, note),
            AppMsg::SetPeerStatus { peer_id, status } => self.set_peer_status(peer_id, status),
            AppMsg::DeletePeer(peer_id) => self.delete_peer(peer_id),
            AppMsg::SyncSessionCompleted(result) => self.finish_sync_pair(result),
        }
        self.sync_ui(&sender);
    }
}

// ── Focus mode logic ──

impl App {
    fn enter_focus(&mut self, note_id: String, sender: &ComponentSender<Self>) {
        self.state.focus_mode = FocusMode::Reading(note_id.clone());
        self.state.selected_note_id = Some(note_id.clone());
        self.overlay_split_view.set_collapsed(true);
        self.content_stack.set_visible_child_name("reading");
        self.load_note_detail(note_id, sender);
    }

    fn exit_focus(&mut self, sender: &ComponentSender<Self>) {
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

    fn start_create_note(&mut self) {
        self.state.focus_mode = FocusMode::Editing(WorkspaceMode::CreateDraft);
        self.state.draft_content.clear();
        self.state.draft_tags_text.clear();
        self.state.status = None;
        self.overlay_split_view.set_collapsed(true);
        self.editing_title_label.set_text("新建笔记");
        self.editing_hint_label
            .set_text("直接记录，不需要先分类。Markdown、清单和引用都可以原样输入。");
        // Stay on "reading" page, just toggle editor to editable
        self.reading_editor.borrow().set_read_only(false);
        self.reading_editor.borrow().set_content("");
    }

    fn start_edit_note(&mut self) {
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

    fn start_reply_to_note(&mut self) {
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

    fn cancel_draft(&mut self, sender: &ComponentSender<Self>) {
        // Switch editor back to read-only
        self.reading_editor.borrow().set_read_only(true);

        match &self.state.focus_mode {
            FocusMode::Editing(WorkspaceMode::ReplyDraft(_))
            | FocusMode::Editing(WorkspaceMode::EditDraft(_)) => {
                if let Some(note_id) = self.state.selected_note_id.clone() {
                    self.state.focus_mode = FocusMode::Reading(note_id.clone());
                    // Reload the note content in read-only mode
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

    fn save_draft(&mut self, sender: &ComponentSender<Self>) {
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

                // Switch editor back to read-only
                self.reading_editor.borrow().set_read_only(true);

                if matches!(mode, WorkspaceMode::ReplyDraft(_)) {
                    self.toast_overlay.add_toast(adw::Toast::new("已发送回复"));
                    // 回到阅读父笔记
                    if let Some(parent_id) = self.state.selected_note_id.clone() {
                        self.state.focus_mode = FocusMode::Reading(parent_id.clone());
                        self.load_note_detail(parent_id, sender);
                    }
                } else {
                    // 新建/编辑 → 进入阅读这条笔记
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

                // 刷新列表
                let query = self.state.search_query.clone();
                if let Ok(home) = load_home(self.core.as_ref(), &query) {
                    self.state.home = home;
                    self.state.sync_selection();
                }
            }
            Err(error) => self.state.status = Some(format!("保存失败: {error}")),
        }
    }

    fn delete_selected_note(&mut self, sender: &ComponentSender<Self>) {
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
}

// ── Browse mode logic ──

impl App {
    fn connect_note_list(&self, sender: &ComponentSender<Self>) {
        let sender_for_activate = sender.input_sender().clone();
        self.list_box.connect_row_activated(move |_, row| {
            let _ = sender_for_activate.send(AppMsg::NoteRowActivated(row.index() as u32));
        });
    }

    fn navigate(&mut self, view: ContentView, sender: &ComponentSender<Self>) {
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

    fn clear_filters(&mut self, sender: &ComponentSender<Self>) {
        self.state.selected_tag = None;
        self.state.search_query.clear();
        self.state.content_view = ContentView::Notes;
        self.state.tag_notes.clear();
        self.state.sync_selection();
        self.refresh_home(sender);
    }

    fn rebuild_list(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let visible = self.state.visible_notes();
        for note in &visible {
            self.list_box.append(&build_note_row(note, sender));
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

    fn load_note_detail(&self, note_id: String, sender: &ComponentSender<Self>) {
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = crate::usecase::load_note_detail(core.as_ref(), &note_id);
            let _ = sender.input_sender().send(AppMsg::NoteDetailLoaded(result));
        });
    }

    fn load_more_notes(&mut self, sender: &ComponentSender<Self>) {
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

    fn finish_loading_more(
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

    fn open_tag_notes(&mut self, tag: String, sender: &ComponentSender<Self>) {
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

    fn refresh_home(&mut self, sender: &ComponentSender<Self>) {
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

// ── UI sync ──

impl App {
    fn sync_ui(&self, sender: &ComponentSender<Self>) {
        self.sync_content_stack();
        self.sync_empty_page();
        self.sync_reading(sender);
        self.sync_editing();
        self.sync_theme_dropdown();
        self.sync_settings(sender);
        self.sync_tags(sender);
        self.sync_timeline(sender);
    }

    fn sync_content_stack(&self) {
        // 如果在 focus 模式，不需要切换 browse 页面
        if !self.state.focus_mode.is_browse() {
            return;
        }

        let is_empty = self.state.visible_notes().is_empty();
        let child_name = match self.state.content_view {
            ContentView::Notes | ContentView::Trash | ContentView::TagNotes => {
                if is_empty {
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
    }

    fn sync_empty_page(&self) {
        let (title, desc) = match self.state.content_view {
            ContentView::Notes if self.state.search_query.is_empty() => (
                "还没有笔记",
                "从左侧点击新建笔记，开始记录你的第一条内容。".to_string(),
            ),
            ContentView::Notes => (
                "没有找到匹配笔记",
                format!(
                    "未检索到与\"{}\"相关的笔记，换个关键词再试试。",
                    self.state.search_query
                ),
            ),
            ContentView::Trash if self.state.search_query.is_empty() => {
                ("回收站是空的", "当前没有已删除笔记。".to_string())
            }
            ContentView::Trash => (
                "回收站中没有匹配项",
                format!("回收站里没有与\"{}\"相关的内容。", self.state.search_query),
            ),
            _ => ("", String::new()),
        };
        self.empty_page.set_title(title);
        self.empty_page.set_description(Some(&desc));
    }

    fn sync_reading(&self, sender: &ComponentSender<Self>) {
        self.reading_context_panel
            .set_visible(self.state.context_panel_open);
        self.reading_editor.borrow_mut().set_content(&self.reading_content());
        self.reading_meta_label.set_text(&self.reading_meta());

        clear_box(&self.reading_tags_box);
        if let Some(detail) = &self.state.selected_note_detail {
            if detail.tags.is_empty() {
                let label = gtk::Label::new(Some("暂无标签"));
                label.add_css_class("caption");
                label.add_css_class("dim-label");
                self.reading_tags_box.append(&label);
            } else {
                for tag in &detail.tags {
                    self.reading_tags_box.append(&tag_chip(tag));
                }
            }
        }

        self.sync_note_section(
            &self.reading_origins_box,
            "无溯源",
            |full| &full.origins,
            sender,
        );
        self.sync_note_section(
            &self.reading_replies_box,
            "无回复",
            |full| &full.replies,
            sender,
        );

        while self.reading_versions_box.observe_children().n_items() > 1 {
            if let Some(child) = self.reading_versions_box.last_child() {
                self.reading_versions_box.remove(&child);
            }
        }
        if let Some(full) = &self.state.selected_note_full {
            if full.other_versions.is_empty() {
                self.reading_versions_box
                    .append(&empty_relation_label("无其他版本"));
            } else {
                for version in &full.other_versions {
                    let note = &version.note;
                    self.reading_versions_box.append(&build_clickable_note_row(
                        note,
                        sender,
                        note.id.clone(),
                    ));
                }
            }
        }
    }

    fn sync_note_section<'a>(
        &'a self,
        container: &gtk::Box,
        empty_title: &str,
        notes: impl Fn(&'a crate::domain::NoteDetailData) -> &'a [synap_core::dto::NoteDTO],
        sender: &ComponentSender<Self>,
    ) {
        while container.observe_children().n_items() > 1 {
            if let Some(child) = container.last_child() {
                container.remove(&child);
            }
        }

        if let Some(full) = &self.state.selected_note_full {
            let items = notes(full);
            if items.is_empty() {
                container.append(&empty_relation_label(empty_title));
            } else {
                for note in items {
                    container.append(&build_clickable_note_row(note, sender, note.id.clone()));
                }
            }
        }
    }

    fn sync_editing(&self) {
        // Only sync content when in editing mode
        if self.state.focus_mode.is_editing() {
            let editor = self.reading_editor.borrow();
            if editor.content() != self.state.draft_content {
                editor.set_content(&self.state.draft_content);
            }
        }
        if self.editing_tags_entry.text().as_str() != self.state.draft_tags_text {
            self.editing_tags_entry
                .set_text(&self.state.draft_tags_text);
        }
    }

    fn sync_theme_dropdown(&self) {
        let idx = self.state.theme.index();
        if self.theme_dropdown.selected() != idx {
            self.theme_dropdown.set_selected(idx);
        }
    }

    fn sync_settings(&self, sender: &ComponentSender<Self>) {
        let listener = &self.state.sync.listener;
        self.sync_listener_row.set_subtitle(&format!(
            "{}{}",
            listener.status,
            listener
                .listen_port
                .map(|port| format!(" · 端口 {port}"))
                .unwrap_or_default()
        ));
        let addresses = if listener.local_addresses.is_empty() {
            "未获取到局域网地址".to_string()
        } else {
            listener.local_addresses.join(", ")
        };
        self.sync_addresses_row.set_subtitle(&addresses);

        self.sync_identity_row.set_subtitle(
            self.state
                .sync
                .local_identity
                .as_ref()
                .map(|id| id.identity.kaomoji_fingerprint.as_str())
                .unwrap_or("—"),
        );
        self.sync_signing_row.set_subtitle(
            self.state
                .sync
                .local_identity
                .as_ref()
                .map(|id| id.signing.kaomoji_fingerprint.as_str())
                .unwrap_or("—"),
        );

        self.sync_error_label
            .set_visible(self.state.sync.error_message.is_some());
        self.sync_error_label
            .set_text(self.state.sync.error_message.as_deref().unwrap_or(""));

        if self.sync_host_entry.text().as_str() != self.state.sync.host_input {
            self.sync_host_entry.set_text(&self.state.sync.host_input);
        }
        if self.sync_port_entry.text().as_str() != self.state.sync.port_input {
            self.sync_port_entry.set_text(&self.state.sync.port_input);
        }

        self.sync_settings_discovered(sender);
        self.sync_settings_connections(sender);
        self.sync_settings_peers(sender);
        self.sync_settings_sessions();
    }

    fn sync_settings_discovered(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.sync_discovered_box);
        if self.state.sync.discovered_peers.is_empty() {
            self.sync_discovered_box.append(&simple_info_row(
                "暂无发现设备",
                "确认设备在同一局域网并已启动监听",
            ));
            return;
        }
        for peer in &self.state.sync.discovered_peers {
            let row = adw::ActionRow::builder()
                .title(&peer.display_name)
                .subtitle(format!("{}:{} · 局域网发现", peer.host, peer.port))
                .build();
            let button = gtk::Button::with_label("配对");
            let s = sender.input_sender().clone();
            let host = peer.host.clone();
            let port = peer.port;
            button.connect_clicked(move |_| {
                let _ = s.send(AppMsg::PairDiscoveredPeer {
                    host: host.clone(),
                    port,
                });
            });
            row.add_suffix(&button);
            self.sync_discovered_box.append(&row);
        }
    }

    fn sync_settings_connections(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.sync_connections_box);
        if self.state.sync.connections.is_empty() {
            self.sync_connections_box.append(&simple_info_row(
                "暂无已保存连接",
                "可手动输入主机地址与端口添加",
            ));
            return;
        }
        for conn in &self.state.sync.connections {
            let row = adw::ActionRow::builder()
                .title(&conn.name)
                .subtitle(&conn.status_message)
                .build();
            let pair_btn = gtk::Button::with_label("配对");
            let ps = sender.input_sender().clone();
            let cid = conn.id.clone();
            pair_btn.connect_clicked(move |_| {
                let _ = ps.send(AppMsg::PairSyncConnection(cid.clone()));
            });
            let del_btn = gtk::Button::with_label("删除");
            del_btn.add_css_class("destructive-action");
            let ds = sender.input_sender().clone();
            let did = conn.id.clone();
            del_btn.connect_clicked(move |_| {
                let _ = ds.send(AppMsg::DeleteSyncConnection(did.clone()));
            });
            row.add_suffix(&del_btn);
            row.add_suffix(&pair_btn);
            self.sync_connections_box.append(&row);
        }
    }

    fn sync_settings_peers(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.sync_peers_box);
        if let Some(peer) = &self.state.sync.pending_trust_peer {
            let row = adw::ActionRow::builder()
                .title("待信任设备")
                .subtitle(format!(
                    "{} · {}",
                    peer.kaomoji_fingerprint,
                    crate::domain::peer_status_label(&peer.status)
                ))
                .build();
            let button = gtk::Button::with_label("信任");
            button.add_css_class("suggested-action");
            let s = sender.input_sender().clone();
            let pk = peer.public_key.clone();
            button.connect_clicked(move |_| {
                let _ = s.send(AppMsg::TrustPeer {
                    public_key: pk.clone(),
                    note: None,
                });
            });
            row.add_suffix(&button);
            self.sync_peers_box.append(&row);
        }
        if self.state.sync.peers.is_empty() {
            self.sync_peers_box.append(&simple_info_row(
                "还没有设备记录",
                "首次配对后会在这里显示公钥与信任状态",
            ));
            return;
        }
        for peer in &self.state.sync.peers {
            let row = adw::ExpanderRow::builder()
                .title(peer.note.as_deref().unwrap_or(&peer.kaomoji_fingerprint))
                .subtitle(crate::domain::peer_status_label(&peer.status))
                .build();

            let note_row = adw::EntryRow::builder().title("备注").build();
            note_row.set_text(peer.note.as_deref().unwrap_or(""));
            let ns = sender.input_sender().clone();
            let nid = peer.id.clone();
            note_row.connect_apply(move |entry| {
                let _ = ns.send(AppMsg::UpdatePeerNote {
                    peer_id: nid.clone(),
                    note: (!entry.text().is_empty()).then(|| entry.text().to_string()),
                });
            });
            row.add_row(&note_row);

            let del_row = adw::ActionRow::builder()
                .title("删除设备记录")
                .subtitle("移除本地记录")
                .activatable(true)
                .build();
            let ds = sender.input_sender().clone();
            let did = peer.id.clone();
            let g = gtk::GestureClick::new();
            g.connect_released(move |_, _, _, _| {
                let _ = ds.send(AppMsg::DeletePeer(did.clone()));
            });
            del_row.add_controller(g);
            row.add_row(&del_row);

            self.sync_peers_box.append(&row);
        }
    }

    fn sync_settings_sessions(&self) {
        clear_box(&self.sync_sessions_box);
        if self.state.sync.recent_sessions.is_empty() {
            self.sync_sessions_box.append(&simple_info_row(
                "暂无同步记录",
                "发起或接收一次同步后会显示在这里",
            ));
            return;
        }
        for session in &self.state.sync.recent_sessions {
            self.sync_sessions_box.append(
                &adw::ActionRow::builder()
                    .title(session.peer_label.as_deref().unwrap_or("未知设备"))
                    .subtitle(format!(
                        "{} · {} · {}",
                        crate::domain::sync_role_label(&session.role),
                        crate::domain::sync_status_label(&session.status),
                        crate::domain::format_timestamp(session.finished_at_ms)
                    ))
                    .build(),
            );
        }
    }

    fn sync_tags(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.tags_flow_box.first_child() {
            self.tags_flow_box.remove(&child);
        }
        for tag in &self.state.all_tags {
            let button = gtk::Button::with_label(&format!("#{tag}"));
            button.add_css_class("pill");
            let tc = tag.clone();
            let s = sender.input_sender().clone();
            button.connect_clicked(move |_| {
                let _ = s.send(AppMsg::TagSelected(tc.clone()));
            });
            self.tags_flow_box.append(&button);
        }
    }

    fn sync_timeline(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.timeline_container.first_child() {
            self.timeline_container.remove(&child);
        }
        for session in &self.state.timeline_sessions {
            let session_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
            session_box.add_css_class("synap-timeline-session");

            let start_time = crate::domain::format_timestamp(session.started_at);
            let end_time = crate::domain::format_timestamp(session.ended_at);
            let header = gtk::Label::new(Some(&format!(
                "{} - {} · {} 条笔记",
                start_time, end_time, session.note_count
            )));
            header.add_css_class("heading");
            header.add_css_class("synap-section-heading");
            header.set_halign(gtk::Align::Start);
            session_box.append(&header);

            let notes_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
            for note in &session.notes {
                notes_box.append(&build_clickable_note_row(note, sender, note.id.clone()));
            }
            session_box.append(&notes_box);

            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.set_margin_top(8);
            separator.set_margin_bottom(8);
            session_box.append(&separator);

            self.timeline_container.append(&session_box);
        }
    }

    fn reading_content(&self) -> String {
        self.state
            .selected_note_detail
            .as_ref()
            .map(|d| render_reading_text(&d.content))
            .unwrap_or_else(|| "请先从列表中打开一条笔记。".to_string())
    }

    fn reading_meta(&self) -> String {
        self.state
            .selected_note_detail
            .as_ref()
            .map(|d| {
                format!(
                    "创建于 {}{}",
                    d.created_at_label,
                    if d.deleted { " · 已删除" } else { "" }
                )
            })
            .unwrap_or_default()
    }
}

// ── Sync operations ──

impl App {
    fn refresh_sync(&mut self, sender: &ComponentSender<Self>) {
        self.state.sync.is_loading = true;
        self.state.sync.error_message = None;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let listener = core.ensure_sync_listener_started(45_172);
            let identity = core.get_local_identity();
            let peers = core.get_peers();
            let sessions = core.get_recent_sync_sessions(Some(10));
            let discovered_peers = core
                .discovered_sync_peers()
                .into_iter()
                .map(Into::into)
                .collect();
            let connections = core.sync_connections();
            let _ = sender.input_sender().send(AppMsg::SyncOverviewLoaded {
                listener,
                identity,
                peers,
                sessions,
                discovered_peers,
                connections,
            });
        });
    }

    fn finish_refresh_sync(
        &mut self,
        listener: Result<corenet::ListenerState, synap_core::error::ServiceError>,
        identity: Result<synap_core::dto::LocalIdentityDTO, synap_core::error::ServiceError>,
        peers: Result<Vec<synap_core::dto::PeerDTO>, synap_core::error::ServiceError>,
        sessions: Result<
            Vec<synap_core::dto::SyncSessionRecordDTO>,
            synap_core::error::ServiceError,
        >,
        discovered_peers: Vec<crate::domain::DiscoveredSyncPeer>,
        connections: Vec<crate::domain::SyncConnectionRecord>,
    ) {
        self.state.sync.is_loading = false;
        self.state.sync.discovered_peers = discovered_peers;
        self.state.sync.connections = connections;
        let mut errors = Vec::new();
        match listener {
            Ok(v) => self.state.sync.listener = v.into(),
            Err(e) => errors.push(format!("监听失败: {e}")),
        }
        match identity {
            Ok(v) => self.state.sync.local_identity = Some(v),
            Err(e) => errors.push(format!("读取本机身份失败: {e}")),
        }
        match peers {
            Ok(v) => self.state.sync.peers = v,
            Err(e) => errors.push(format!("读取设备列表失败: {e}")),
        }
        match sessions {
            Ok(v) => self.state.sync.recent_sessions = v,
            Err(e) => errors.push(format!("读取同步统计失败: {e}")),
        }
        self.state.sync.error_message = (!errors.is_empty()).then(|| errors.join("\n"));
    }

    fn add_sync_connection(&mut self) {
        let host = self.state.sync.host_input.trim().to_string();
        let port = self.state.sync.port_input.trim().parse::<u16>();
        match port {
            Ok(port) => match self.core.save_sync_connection(&host, port) {
                Ok(record) => {
                    self.state.sync.connections.retain(|c| c.id != record.id);
                    self.state.sync.connections.push(record);
                    self.state.sync.host_input.clear();
                    self.state.sync.port_input.clear();
                    self.state.sync.error_message = None;
                }
                Err(e) => self.state.sync.error_message = Some(format!("保存连接失败: {e}")),
            },
            Err(_) => self.state.sync.error_message = Some("端口必须是有效数字".to_string()),
        }
    }

    fn delete_sync_connection(&mut self, id: &str) {
        match self.core.delete_sync_connection(id) {
            Ok(()) => {
                self.state.sync.connections.retain(|c| c.id != id);
                self.state.sync.error_message = None;
            }
            Err(e) => self.state.sync.error_message = Some(format!("删除连接失败: {e}")),
        }
    }

    fn start_sync_pair(&mut self, host: String, port: u16, sender: &ComponentSender<Self>) {
        self.state.sync.is_pairing = true;
        self.state.sync.error_message = None;
        let core = self.core.clone();
        let sender = sender.clone();
        gtk::glib::spawn_future_local(async move {
            let result = core.connect_and_sync(&host, port);
            let _ = sender
                .input_sender()
                .send(AppMsg::SyncSessionCompleted(result));
        });
    }

    fn finish_sync_pair(
        &mut self,
        result: Result<synap_core::dto::SyncSessionDTO, synap_core::error::ServiceError>,
    ) {
        self.state.sync.is_pairing = false;
        match result {
            Ok(session) => {
                self.state.sync.pending_trust_peer = (session.status
                    == synap_core::dto::SyncStatusDTO::PendingTrust)
                    .then_some(session.peer.clone());
                self.state.sync.error_message = None;
            }
            Err(e) => self.state.sync.error_message = Some(format!("配对失败: {e}")),
        }
    }

    fn trust_peer(&mut self, public_key: Vec<u8>, note: Option<String>) {
        self.state.sync.is_managing_peer = true;
        match self.core.trust_peer(&public_key, note) {
            Ok(peer) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.pending_trust_peer = None;
                self.state.sync.peers.retain(|p| p.id != peer.id);
                self.state.sync.peers.push(peer);
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("信任对端失败: {e}"));
            }
        }
    }

    fn update_peer_note(&mut self, peer_id: String, note: Option<String>) {
        self.state.sync.is_managing_peer = true;
        match self.core.update_peer_note(&peer_id, note) {
            Ok(peer) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.peers.retain(|p| p.id != peer.id);
                self.state.sync.peers.push(peer.clone());
                self.state.sync.peer_note_draft = peer.note.clone().unwrap_or_default();
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("更新设备备注失败: {e}"));
            }
        }
    }

    fn set_peer_status(&mut self, peer_id: String, status: synap_core::dto::PeerTrustStatusDTO) {
        self.state.sync.is_managing_peer = true;
        match self.core.set_peer_status(&peer_id, status) {
            Ok(peer) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.peers.retain(|p| p.id != peer.id);
                self.state.sync.peers.push(peer);
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("更新设备状态失败: {e}"));
            }
        }
    }

    fn delete_peer(&mut self, peer_id: String) {
        self.state.sync.is_managing_peer = true;
        match self.core.delete_peer(&peer_id) {
            Ok(()) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.peers.retain(|p| p.id != peer_id);
                self.state.sync.error_message = None;
            }
            Err(e) => {
                self.state.sync.is_managing_peer = false;
                self.state.sync.error_message = Some(format!("删除设备失败: {e}"));
            }
        }
    }
}

// ── Utility functions ──

fn render_reading_text(content: &str) -> String {
    let rendered = content
        .lines()
        .map(render_reading_line)
        .collect::<Vec<_>>()
        .join("\n");

    if rendered.trim().is_empty() {
        "空白笔记".to_string()
    } else {
        rendered
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in raw.split([',', '，']) {
        let trimmed = tag.trim();
        if trimmed.is_empty() || tags.iter().any(|existing: &String| existing == trimmed) {
            continue;
        }
        tags.push(trimmed.to_string());
    }
    tags
}

fn compact_single_line(content: &str, max_chars: usize) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
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

fn render_reading_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if trimmed.is_empty() {
        return String::new();
    }

    let (prefix, mut text) = if let Some(rest) = trimmed.strip_prefix("> ") {
        ("│ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        ("☐ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        ("☑ ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("- ") {
        ("• ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("* ") {
        ("• ", rest)
    } else {
        ("", trimmed)
    };

    while let Some(rest) = text.strip_prefix('#') {
        text = rest.trim_start();
    }

    format!("{indent}{prefix}{}", strip_inline_markdown(text))
}

fn strip_inline_markdown(text: &str) -> String {
    text.replace("***", "")
        .replace("**", "")
        .replace('*', "")
        .replace("~~", "")
        .replace("==", "")
        .replace("<u>", "")
        .replace("</u>", "")
}

fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn empty_relation_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label
}

fn simple_info_row(title: &str, subtitle: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build()
}
