pub mod browse;
pub mod focus;
pub mod message;
pub mod sync_ops;
pub mod ui_sync;

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


