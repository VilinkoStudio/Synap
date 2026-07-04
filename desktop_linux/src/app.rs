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
        note_widgets::{build_clickable_note_row, tag_chip},
        shell::build_content_pages,
        theme::apply_theme,
        util::{compact_single_line, parse_tags, render_reading_text},
    },
    usecase::load_home,
};

/// 阅读视图的 widget 引用
pub struct ReadingWidgets {
    pub context_panel: gtk::ScrolledWindow,
    pub editor: RefCell<WysiwygEditor>,
    pub tags_box: gtk::Box,
    pub meta_label: gtk::Label,
    pub origins_box: gtk::Box,
    pub replies_box: gtk::Box,
    pub versions_box: gtk::Box,
}

/// 编辑模式的 widget 引用
pub struct EditingWidgets {
    pub title_label: gtk::Label,
    pub hint_label: gtk::Label,
    pub tags_entry: gtk::Entry,
    pub recommend_tags_box: gtk::Box,
}

/// 笔记列表的 widget 引用
pub struct ListWidgets {
    pub empty_page: adw::StatusPage,
}

/// 设置页的 widget 引用
pub struct SettingsWidgets {
    pub theme_dropdown: gtk::DropDown,
    pub listener_row: adw::ActionRow,
    pub addresses_row: adw::ActionRow,
    pub identity_row: adw::ActionRow,
    pub signing_row: adw::ActionRow,
    pub error_label: gtk::Label,
    // Relay
    pub relay_base_url_entry: gtk::Entry,
    pub relay_api_key_entry: gtk::Entry,
    pub relay_status_label: gtk::Label,
    // Connections
    pub host_entry: gtk::Entry,
    pub port_entry: gtk::Entry,
    pub discovered_box: gtk::Box,
    pub connections_box: gtk::Box,
    pub peers_box: gtk::Box,
    pub sessions_box: gtk::Box,
}

pub struct App {
    core: Rc<dyn DesktopCore>,
    state: AppState,
    search_debounce: Option<gtk::glib::SourceId>,
    toast_overlay: adw::ToastOverlay,
    overlay_split_view: adw::OverlaySplitView,
    content_stack: gtk::Stack,

    // 分组的 widget 引用
    list: ListWidgets,
    reading: ReadingWidgets,
    editing: EditingWidgets,
    settings: SettingsWidgets,
    home_flow_box: gtk::FlowBox,
    home_tag_box: gtk::Box,
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
                                    gtk::Label { set_label: "笔记", set_xalign: 0.0, set_hexpand: true }
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
                                    set_text: model.editing.title_label.text().as_str(),
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
                                    set_visible: model.state.home.tag_filter.tag_filter_enabled || !model.state.search_query.is_empty(),
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

        let default_filter = crate::domain::TagFilter::default();
        match load_home(core.as_ref(), "", &default_filter) {
            Ok(home) => {
                state.home = home;
                state.sync_selection();
            }
            Err(error) => {
                state.status = Some(format!("初始化失败: {error}"));
            }
        }

        apply_theme(state.theme);

        // Load relay config on startup
        let (relay_base_url, relay_api_key) = core.get_relay_config();
        state.sync.relay_base_url = relay_base_url;
        state.sync.relay_api_key = relay_api_key;

        let toast_overlay = adw::ToastOverlay::new();
        let overlay_split_view = adw::OverlaySplitView::new();
        let pages = build_content_pages(&state, sender.input_sender());
        let content_stack = pages.content_stack.clone();

        let model = App {
            core: core.clone(),
            state,
            search_debounce: None,
            toast_overlay: toast_overlay.clone(),
            overlay_split_view: overlay_split_view.clone(),
            content_stack: pages.content_stack,

            list: ListWidgets {
                empty_page: pages.empty_page,
            },
            reading: ReadingWidgets {
                context_panel: pages.reading_context_panel,
                editor: RefCell::new(pages.reading_editor),
                tags_box: pages.reading_tags_box,
                meta_label: pages.reading_meta_label,
                origins_box: pages.reading_origins_box,
                replies_box: pages.reading_replies_box,
                versions_box: pages.reading_versions_box,
            },
            editing: EditingWidgets {
                title_label: pages.editing_title_label,
                hint_label: pages.editing_hint_label,
                tags_entry: pages.editing_tags_entry,
                recommend_tags_box: pages.recommend_tags_box,
            },
            home_flow_box: pages.home_flow_box,
            home_tag_box: pages.home_tag_box,
            settings: SettingsWidgets {
                theme_dropdown: pages.theme_dropdown,
                listener_row: pages.sync_listener_row,
                addresses_row: pages.sync_addresses_row,
                identity_row: pages.sync_identity_row,
                signing_row: pages.sync_signing_row,
                error_label: pages.sync_error_label,
                relay_base_url_entry: pages.relay_base_url_entry,
                relay_api_key_entry: pages.relay_api_key_entry,
                relay_status_label: pages.relay_status_label,
                host_entry: pages.sync_host_entry,
                port_entry: pages.sync_port_entry,
                discovered_box: pages.sync_discovered_box,
                connections_box: pages.sync_connections_box,
                peers_box: pages.sync_peers_box,
                sessions_box: pages.sync_sessions_box,
            },
        };

        let widgets = view_output!();
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
            AppMsg::DraftContentChanged(value) => {
                self.state.draft_content = value;
                // Trigger tag recommendations when content changes
                if self.state.focus_mode.is_editing() && self.state.draft_content.len() > 20 {
                    let core = self.core.clone();
                    let content = self.state.draft_content.clone();
                    let sender = sender.clone();
                    gtk::glib::spawn_future_local(async move {
                        let result = core.recommend_tags(&content, 5);
                        let _ = sender.input_sender().send(AppMsg::TagRecommendationsLoaded(result));
                    });
                }
            }
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

            // ── Tag filters ──
            AppMsg::ToggleTagFilter(tag) => self.toggle_tag_filter(tag, &sender),
            AppMsg::ToggleUntaggedFilter => self.toggle_untagged_filter(&sender),
            AppMsg::ToggleAllTags => self.toggle_all_tags(&sender),
            AppMsg::TagRecommendationsLoaded(result) => match result {
                Ok(tags) => {
                    self.state.recommended_tags = tags;
                }
                Err(_) => { /* silently ignore recommendation failures */ }
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
            AppMsg::AddSyncConnection => self.add_sync_connection(&sender),
            AppMsg::DeleteSyncConnection(id) => self.delete_sync_connection(&id, &sender),
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
            AppMsg::TrustPeer { public_key, note } => {
                self.trust_peer(public_key, note, &sender)
            }
            AppMsg::UpdatePeerNote { peer_id, note } => {
                self.update_peer_note(peer_id, note, &sender)
            }
            AppMsg::DeletePeer(peer_id) => self.delete_peer(peer_id, &sender),
            AppMsg::SetPeerStatus { peer_id, status } => {
                self.set_peer_status(peer_id, status, &sender)
            }
            AppMsg::DismissPendingTrustPrompt => self.dismiss_pending_trust_prompt(),
            AppMsg::SyncSessionCompleted(result) => self.finish_sync_pair(result),

            // ── Relay config ──
            AppMsg::UpdateRelayBaseUrl(value) => self.update_relay_base_url(&value),
            AppMsg::UpdateRelayApiKey(value) => self.update_relay_api_key(&value),
            AppMsg::SaveRelayConfig => self.save_relay_config(&sender),
            AppMsg::RelayConfigSaved(result) => self.finish_save_relay_config(result),

            // ── Relay operations ──
            AppMsg::FetchRelayUpdates => self.fetch_relay_updates(&sender),
            AppMsg::PushRelayUpdates => self.push_relay_updates(&sender),
            AppMsg::RelayFetchCompleted(result) => {
                self.finish_fetch_relay_updates(result);
                self.refresh_sync(&sender);
            }
            AppMsg::RelayPushCompleted(result) => {
                self.finish_push_relay_updates(result);
                self.refresh_sync(&sender);
            }

            // ── Async operation results ──
            AppMsg::NoteSaved(result) => match result {
                Ok(note) => {
                    self.state.draft_content.clear();
                    self.state.draft_tags_text.clear();
                    self.state.search_query.clear();
                    self.state.status = None;
                    self.reading.editor.borrow().set_read_only(true);

                    // Refresh lists in background
                    let query = self.state.search_query.clone();
                    let tag_filter = self.state.home.tag_filter.clone();
                    let core = self.core.clone();
                    let s = sender.clone();
                    gtk::glib::spawn_future_local(async move {
                        let result = load_home(core.as_ref(), &query, &tag_filter);
                        let _ = s.input_sender().send(AppMsg::HomeRefreshed(result));
                    });

                    // Determine toast and next focus
                    let is_reply = matches!(
                        self.state.focus_mode,
                        FocusMode::Editing(WorkspaceMode::ReplyDraft(_))
                    );
                    if is_reply {
                        self.toast_overlay.add_toast(adw::Toast::new("已发送回复"));
                        if let Some(parent_id) = self.state.selected_note_id.clone() {
                            self.state.focus_mode = FocusMode::Reading(parent_id.clone());
                            self.load_note_detail(parent_id, &sender);
                        }
                    } else {
                        let is_create = matches!(
                            self.state.focus_mode,
                            FocusMode::Editing(WorkspaceMode::CreateDraft)
                        );
                        let msg = if is_create { "已创建笔记" } else { "已更新笔记" };
                        self.toast_overlay.add_toast(adw::Toast::new(msg));
                        self.state.focus_mode = FocusMode::Reading(note.id.clone());
                        self.state.selected_note_id = Some(note.id.clone());
                        self.load_note_detail(note.id, &sender);
                    }
                }
                Err(error) => self.state.status = Some(format!("保存失败: {error}")),
            },
            AppMsg::NoteDeleted(result) => match result {
                Ok(()) => {
                    self.toast_overlay.add_toast(adw::Toast::new("已删除笔记"));
                    self.exit_focus(&sender);
                    self.refresh_home(&sender);
                }
                Err(error) => self.state.status = Some(format!("删除失败: {error}")),
            },
            AppMsg::NoteRestored(result) => match result {
                Ok(()) => {
                    self.toast_overlay.add_toast(adw::Toast::new("已恢复笔记"));
                    self.refresh_home(&sender);
                }
                Err(error) => self.state.status = Some(format!("恢复失败: {error}")),
            },
            AppMsg::HomeRefreshed(result) => match result {
                Ok(home) => {
                    self.state.home = home;
                    self.state.sync_selection();
                    self.state.status = None;
                    self.rebuild_list(&sender);
                }
                Err(error) => self.state.status = Some(format!("加载失败: {error}")),
            },
            AppMsg::SyncConnectionSaved(result) => match result {
                Ok(record) => {
                    self.state.sync.connections.retain(|c| c.id != record.id);
                    self.state.sync.connections.push(record);
                    self.state.sync.host_input.clear();
                    self.state.sync.port_input.clear();
                    self.state.sync.error_message = None;
                }
                Err(e) => self.state.sync.error_message = Some(format!("保存连接失败: {e}")),
            },
            AppMsg::SyncConnectionDeleted(result) => match result {
                Ok(()) => {
                    // The caller already removed from local state
                    self.state.sync.error_message = None;
                }
                Err(e) => self.state.sync.error_message = Some(format!("删除连接失败: {e}")),
            },
            AppMsg::PeerTrusted(result) => match result {
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
            },
            AppMsg::PeerNoteUpdated(result) => match result {
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
            },
            AppMsg::PeerDeleted(result) => match result {
                Ok(()) => {
                    self.state.sync.is_managing_peer = false;
                    self.state.sync.error_message = None;
                }
                Err(e) => {
                    self.state.sync.is_managing_peer = false;
                    self.state.sync.error_message = Some(format!("删除设备失败: {e}"));
                }
            },
        }
        self.sync_ui(&sender);
    }
}


