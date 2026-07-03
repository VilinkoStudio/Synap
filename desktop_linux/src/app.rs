pub mod browse;
pub mod focus;
pub mod message;
pub mod sync_ops;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use relm4::prelude::*;

#[allow(unused_imports)]
use crate::{
    app::message::AppMsg,
    core::DesktopCore,
    domain::{AppState, ContentView, FocusMode, Theme, WorkspaceMode},
    ui::{
        editor::WysiwygEditor,
        note_widgets::{build_clickable_note_row, build_note_row, tag_chip},
        shell::build_content_pages,
        theme::apply_theme,
        util::{compact_single_line, parse_tags, render_reading_text},
    },
    usecase::load_home,
};

pub struct App {
    core: Rc<dyn DesktopCore>,
    state: AppState,
    search_debounce: Option<gtk::glib::SourceId>,
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
            search_debounce: None,
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

        // ── Global keyboard shortcuts ──
        let key_ctrl = gtk::EventControllerKey::new();
        let sender_keys = sender.input_sender().clone();
        key_ctrl.connect_key_pressed(move |_, key, _, modifiers| {
            let ctrl = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            match (ctrl, key) {
                // Ctrl+N → new note
                (true, gtk::gdk::Key::n) => {
                    let _ = sender_keys.send(AppMsg::StartCreateNote);
                    gtk::glib::Propagation::Stop
                }
                // Escape → exit focus mode
                (false, gtk::gdk::Key::Escape) => {
                    let _ = sender_keys.send(AppMsg::ExitFocus);
                    gtk::glib::Propagation::Stop
                }
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        model.toast_overlay.add_controller(key_ctrl);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            // ── Browse navigation ──
            AppMsg::Navigate(view) => self.navigate(view, &sender),
            AppMsg::SearchChanged(query) => {
                self.state.search_query = query;
                // Debounce search: cancel previous timer, schedule new one (200ms)
                if let Some(source) = self.search_debounce.take() {
                    source.remove();
                }
                let s = sender.input_sender().clone();
                let source = gtk::glib::timeout_add_local(
                    std::time::Duration::from_millis(200),
                    move || {
                        let _ = s.send(AppMsg::RunSearch);
                        gtk::glib::ControlFlow::Break
                    },
                );
                self.search_debounce = Some(source);
            }
            AppMsg::RunSearch => self.refresh_home(&sender),
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
            AppMsg::DraftContentChanged(value) => self.state.draft_content = value,
            AppMsg::DraftTagsChanged(value) => self.state.draft_tags_text = value,
            AppMsg::SaveDraft => self.save_draft(&sender),
            AppMsg::CancelDraft => self.cancel_draft(&sender),

            // ── Note operations (from reading toolbar) ──
            AppMsg::EditNote => self.start_edit_note(),
            AppMsg::ReplyToNote => self.start_reply_to_note(),
            AppMsg::DeleteNote => self.confirm_delete_note(&sender),
            AppMsg::ConfirmDeleteNote => self.delete_selected_note(&sender),
            AppMsg::RestoreNote(id) => self.restore_note(&id, &sender),

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
            AppMsg::DeletePeer(peer_id) => self.delete_peer(peer_id),
            AppMsg::SyncSessionCompleted(result) => self.finish_sync_pair(result),
        }
        self.sync_ui(&sender);
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

// ── GTK helpers ──

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
