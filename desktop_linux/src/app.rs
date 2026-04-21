use std::rc::Rc;

use adw::prelude::*;
use relm4::prelude::*;

use crate::core::DesktopCore;
use crate::domain::{AppState, ContentView, NoteDetailData, NoteLayout, Theme};
use crate::usecase::load_home;
use synap_core::{dto::NoteDTO, error::ServiceError};

pub struct App {
    core: Rc<dyn DesktopCore>,
    state: AppState,
    toast_overlay: adw::ToastOverlay,
    list_box: gtk::ListBox,
    content_stack: gtk::Stack,
    empty_page: adw::StatusPage,
    detail_content_row: adw::ActionRow,
    detail_tags_row: adw::ActionRow,
    detail_meta_row: adw::ActionRow,
    detail_origins_box: gtk::Box,
    detail_replies_box: gtk::Box,
    detail_versions_box: gtk::Box,
    detail_origins_label: gtk::Label,
    detail_replies_label: gtk::Label,
    detail_versions_label: gtk::Label,
    theme_dropdown: gtk::DropDown,
    tags_flow_box: gtk::FlowBox,
    timeline_container: gtk::Box,
    layout_stack: gtk::Stack,
    flow_box: gtk::FlowBox,
}

#[derive(Debug)]
pub enum AppMsg {
    Navigate(ContentView),
    SearchChanged(String),
    LayoutChanged(NoteLayout),
    DeleteNote,
    SaveNote {
        id: Option<String>,
        content: String,
        tags: Vec<String>,
    },
    SaveReply {
        parent_id: String,
        content: String,
        tags: Vec<String>,
    },
    CreateNote,
    EditNote,
    ReplyToNote,
    ThemeChanged(Theme),
    NoteSelected(u32),
    NoteActivated(u32),
    NoteDetailLoaded(Result<NoteDetailData, ServiceError>),
    OpenNoteDetail(String),
    LoadMoreNotes,
    MoreNotesLoaded(Result<(Vec<NoteDTO>, Option<String>, bool), ServiceError>),
    TagSelected(String),
    TagsLoaded(Result<Vec<String>, ServiceError>),
    TagNotesLoaded(Result<Vec<NoteDTO>, ServiceError>),
    TagSuggestionsLoaded(Result<Vec<String>, ServiceError>),
    ClearFilters,
    TimelineLoaded(Result<Vec<synap_core::dto::TimelineSessionDTO>, ServiceError>),
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
            set_default_size: (816, 552),

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                adw::OverlaySplitView {
                    set_sidebar_width_fraction: 0.22,
                    set_min_sidebar_width: 220.0,
                    set_max_sidebar_width: 320.0,

                    #[wrap(Some)]
                    set_sidebar = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            adw::HeaderBar {
                                set_show_end_title_buttons: false,

                                #[wrap(Some)]
                                set_title_widget = &gtk::Label {
                                    set_label: "Synap",
                                    add_css_class: "title-1"
                                }
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 6,
                                set_margin_top: 12,
                                set_margin_start: 12,
                                set_margin_end: 12,
                                set_margin_bottom: 12,

                                gtk::Button {
                                    set_label: "新建笔记",
                                    add_css_class: "pill",
                                    add_css_class: "suggested-action",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::CreateNote);
                                    }
                                },

                                gtk::Button {
                                    set_label: "笔记列表",
                                    #[watch]
                                    set_css_classes: if model.state.content_view == ContentView::Notes {
                                        &["flat", "active"]
                                    } else {
                                        &["flat"]
                                    },
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::Navigate(ContentView::Notes));
                                    }
                                },

                                gtk::Button {
                                    set_label: "回收站",
                                    #[watch]
                                    set_css_classes: if model.state.content_view == ContentView::Trash {
                                        &["flat", "active"]
                                    } else {
                                        &["flat"]
                                    },
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::Navigate(ContentView::Trash));
                                    }
                                },

                                gtk::Button {
                                    set_label: "标签",
                                    #[watch]
                                    set_css_classes: if model.state.content_view == ContentView::Tags || model.state.content_view == ContentView::TagNotes {
                                        &["flat", "active"]
                                    } else {
                                        &["flat"]
                                    },
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::Navigate(ContentView::Tags));
                                    }
                                },

                                gtk::Button {
                                    set_label: "时间线",
                                    #[watch]
                                    set_css_classes: if model.state.content_view == ContentView::Timeline {
                                        &["flat", "active"]
                                    } else {
                                        &["flat"]
                                    },
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::Navigate(ContentView::Timeline));
                                    }
                                },

                                gtk::Box {
                                    set_vexpand: true
                                },

                                gtk::Button {
                                    set_label: "设置",
                                    #[watch]
                                    set_css_classes: if model.state.content_view == ContentView::Settings {
                                        &["flat", "active"]
                                    } else {
                                        &["flat"]
                                    },
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::Navigate(ContentView::Settings));
                                    }
                                }
                            }
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            adw::HeaderBar {
                                #[wrap(Some)]
                                set_title_widget = &adw::WindowTitle {
                                    set_title: "Synap",
                                    #[watch]
                                    set_subtitle: model.state.content_view.title()
                                }
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                set_margin_top: 12,
                                set_margin_bottom: 6,
                                set_margin_start: 18,
                                set_margin_end: 18,
                                #[watch]
                                set_visible: model.state.content_view != ContentView::Settings,

                                gtk::SearchEntry {
                                    set_placeholder_text: Some("搜索内容或标签"),
                                    set_hexpand: true,
                                    set_max_width_chars: 40,
                                    connect_search_changed[sender] => move |entry| {
                                        sender.input(AppMsg::SearchChanged(entry.text().to_string()));
                                    }
                                },

                                gtk::Button {
                                    set_label: "清除筛选",
                                    add_css_class: "suggested-action",
                                    #[watch]
                                    set_visible: model.state.content_view == ContentView::TagNotes || !model.state.search_query.is_empty(),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppMsg::ClearFilters);
                                    }
                                },

                                gtk::DropDown {
                                    set_model: Some(&gtk::StringList::new(&[
                                        NoteLayout::Waterfall.label(),
                                        NoteLayout::List.label()
                                    ])),
                                    #[watch]
                                    set_selected: model.state.layout.index(),
                                    connect_selected_notify[sender] => move |dropdown| {
                                        sender.input(AppMsg::LayoutChanged(NoteLayout::from_index(dropdown.selected())));
                                    }
                                }
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
                            content_stack -> gtk::Stack {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_margin_start: 12,
                                set_margin_end: 12,
                                set_margin_bottom: 12,
                            }
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

        let toast_overlay = adw::ToastOverlay::new();
        let list_box = gtk::ListBox::new();
        list_box.set_css_classes(&["boxed-list"]);
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.set_vexpand(true);

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

        // Build Stack and its children imperatively
        let content_stack = gtk::Stack::new();
        content_stack.set_hexpand(true);
        content_stack.set_vexpand(true);
        content_stack.set_margin_start(12);
        content_stack.set_margin_end(12);
        content_stack.set_margin_bottom(12);

        // Notes page with layout switcher
        let layout_stack = gtk::Stack::new();
        
        // List layout
        let list_scroller = gtk::ScrolledWindow::new();
        list_scroller.set_child(Some(&list_box));
        layout_stack.add_named(&list_scroller, Some("list"));
        
        // Waterfall layout
        let waterfall_scroller = gtk::ScrolledWindow::new();
        waterfall_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        let flow_box = gtk::FlowBox::new();
        flow_box.set_selection_mode(gtk::SelectionMode::Single);
        flow_box.set_homogeneous(false);
        flow_box.set_max_children_per_line(2);
        flow_box.set_column_spacing(12);
        flow_box.set_row_spacing(12);
        flow_box.set_margin_start(12);
        flow_box.set_margin_end(12);
        flow_box.set_margin_top(12);
        flow_box.set_margin_bottom(12);
        flow_box.set_hexpand(true);
        waterfall_scroller.set_child(Some(&flow_box));
        layout_stack.add_named(&waterfall_scroller, Some("waterfall"));
        
        // Main scroller for infinite scroll (works for both layouts)
        let notes_scroller = gtk::ScrolledWindow::new();
        notes_scroller.set_child(Some(&layout_stack));
        
        // Infinite scroll setup
        let sender_scroll = sender.input_sender().clone();
        notes_scroller.vadjustment().connect_value_changed(move |adj| {
            let upper = adj.upper();
            let page_size = adj.page_size();
            let value = adj.value();
            
            // Trigger load when within 100px of bottom
            if upper > page_size && value >= upper - page_size - 100.0 {
                let _ = sender_scroll.send(AppMsg::LoadMoreNotes);
            }
        });
        
        content_stack.add_named(&notes_scroller, Some("notes"));

        // Empty page
        let empty_page = adw::StatusPage::new();
        empty_page.set_icon_name(Some("document-edit-symbolic"));
        content_stack.add_named(&empty_page, Some("empty"));

        // Detail page
        let detail_clamp = adw::Clamp::builder()
            .maximum_size(800)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let detail_box = gtk::Box::new(gtk::Orientation::Vertical, 24);

        let detail_group = adw::PreferencesGroup::builder().title("笔记内容").build();

        let detail_content_row = adw::ActionRow::builder()
            .title("内容")
            .subtitle_selectable(true)
            .build();
        let detail_tags_row = adw::ActionRow::builder().title("标签").build();
        let detail_meta_row = adw::ActionRow::builder().title("创建时间").build();

        detail_group.add(&detail_content_row);
        detail_group.add(&detail_tags_row);
        detail_group.add(&detail_meta_row);

        let detail_buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        detail_buttons.set_halign(gtk::Align::End);

        let reply_button = gtk::Button::with_label("回复笔记");
        let sender_reply = sender.input_sender().clone();
        reply_button.connect_clicked(move |_| {
            let _ = sender_reply.send(AppMsg::ReplyToNote);
        });

        let edit_button = gtk::Button::with_label("编辑笔记");
        let sender_edit = sender.input_sender().clone();
        edit_button.connect_clicked(move |_| {
            let _ = sender_edit.send(AppMsg::EditNote);
        });

        let delete_button = gtk::Button::with_label("删除笔记");
        delete_button.add_css_class("destructive-action");
        let sender_delete = sender.input_sender().clone();
        delete_button.connect_clicked(move |_| {
            let _ = sender_delete.send(AppMsg::DeleteNote);
        });

        detail_buttons.append(&reply_button);
        detail_buttons.append(&edit_button);
        detail_buttons.append(&delete_button);

        // Origins section
        let detail_origins_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let origins_label = gtk::Label::new(Some("溯源链"));
        origins_label.add_css_class("heading");
        origins_label.set_halign(gtk::Align::Start);
        detail_origins_box.append(&origins_label);

        // Replies section
        let detail_replies_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let replies_label = gtk::Label::new(Some("回复"));
        replies_label.add_css_class("heading");
        replies_label.set_halign(gtk::Align::Start);
        detail_replies_box.append(&replies_label);

        // Versions section
        let detail_versions_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let versions_label = gtk::Label::new(Some("其他版本"));
        versions_label.add_css_class("heading");
        versions_label.set_halign(gtk::Align::Start);
        detail_versions_box.append(&versions_label);

        detail_box.append(&detail_group);
        detail_box.append(&detail_origins_box);
        detail_box.append(&detail_replies_box);
        detail_box.append(&detail_versions_box);
        detail_box.append(&detail_buttons);
        detail_clamp.set_child(Some(&detail_box));

        let detail_scroller = gtk::ScrolledWindow::new();
        detail_scroller.set_child(Some(&detail_clamp));
        content_stack.add_named(&detail_scroller, Some("detail"));

        // Settings page
        let settings_page = adw::PreferencesPage::new();
        let settings_group = adw::PreferencesGroup::builder().title("外观").build();

        let theme_row = adw::ActionRow::builder()
            .title("主题")
            .subtitle("选择应用的颜色主题")
            .build();

        let theme_dropdown = gtk::DropDown::from_strings(&["跟随系统", "浅色", "深色"]);
        theme_dropdown.set_selected(state.theme.index());
        let sender_theme = sender.input_sender().clone();
        theme_dropdown.connect_selected_notify(move |dropdown| {
            let _ = sender_theme.send(AppMsg::ThemeChanged(Theme::from_index(dropdown.selected())));
        });
        theme_row.add_suffix(&theme_dropdown);
        settings_group.add(&theme_row);
        settings_page.add(&settings_group);
        content_stack.add_named(&settings_page, Some("settings"));

        // Tags page
        let tags_page = gtk::ScrolledWindow::new();
        let tags_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        tags_box.set_margin_top(24);
        tags_box.set_margin_bottom(24);
        tags_box.set_margin_start(24);
        tags_box.set_margin_end(24);
        
        let tags_label = gtk::Label::new(Some("所有标签"));
        tags_label.add_css_class("heading");
        tags_label.set_halign(gtk::Align::Start);
        tags_box.append(&tags_label);
        
        let tags_flow_box = gtk::FlowBox::new();
        tags_flow_box.set_selection_mode(gtk::SelectionMode::None);
        tags_flow_box.set_homogeneous(true);
        tags_flow_box.set_max_children_per_line(3);
        tags_flow_box.set_min_children_per_line(1);
        tags_flow_box.set_column_spacing(12);
        tags_flow_box.set_row_spacing(12);
        
        // We'll populate tags dynamically in sync_ui
        tags_box.append(&tags_flow_box);
        tags_page.set_child(Some(&tags_box));
        content_stack.add_named(&tags_page, Some("tags"));

        // Timeline page
        let timeline_page = gtk::ScrolledWindow::new();
        let timeline_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        timeline_box.set_margin_top(24);
        timeline_box.set_margin_bottom(24);
        timeline_box.set_margin_start(24);
        timeline_box.set_margin_end(24);
        
        let timeline_container = gtk::Box::new(gtk::Orientation::Vertical, 24);
        timeline_box.append(&timeline_container);
        
        timeline_page.set_child(Some(&timeline_box));
        content_stack.add_named(&timeline_page, Some("timeline"));

        // Set initial visible child
        let is_empty = state.visible_notes().is_empty();
        let initial_child = match state.content_view {
            ContentView::NoteDetail => "detail",
            ContentView::Settings => "settings",
            _ => {
                if is_empty {
                    "empty"
                } else {
                    "notes"
                }
            }
        };
        content_stack.set_visible_child_name(initial_child);

        let model = App {
            core: core.clone(),
            state,
            toast_overlay: toast_overlay.clone(),
            list_box: list_box.clone(),
            content_stack: content_stack.clone(),
            empty_page: empty_page.clone(),
            detail_content_row: detail_content_row.clone(),
            detail_tags_row: detail_tags_row.clone(),
            detail_meta_row: detail_meta_row.clone(),
            detail_origins_box: detail_origins_box.clone(),
            detail_replies_box: detail_replies_box.clone(),
            detail_versions_box: detail_versions_box.clone(),
            detail_origins_label: origins_label.clone(),
            detail_replies_label: replies_label.clone(),
            detail_versions_label: versions_label.clone(),
            theme_dropdown: theme_dropdown.clone(),
            tags_flow_box: tags_flow_box.clone(),
            timeline_container: timeline_container.clone(),
            layout_stack: layout_stack.clone(),
            flow_box: flow_box.clone(),
        };

        let widgets = view_output!();

        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let sender_for_select = sender.input_sender().clone();
        list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let _ = sender_for_select.send(AppMsg::NoteSelected(row.index() as u32));
            }
        });

        let sender_for_activate = sender.input_sender().clone();
        list_box.connect_row_activated(move |_, row| {
            let _ = sender_for_activate.send(AppMsg::NoteActivated(row.index() as u32));
        });

        model.rebuild_list(&sender);
        model.sync_ui(&sender);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Navigate(view) => {
                if self.state.content_view != view {
                    self.state.content_view = view;
                    self.state.sync_selection();
                    self.rebuild_list(&sender);

                    if view == ContentView::Tags {
                        let core = self.core.clone();
                        let sender = sender.clone();
                        gtk::glib::spawn_future_local(async move {
                            let result = core.get_all_tags();
                            let _ = sender.input_sender().send(AppMsg::TagsLoaded(result));
                        });
                    }

                    if view == ContentView::Timeline {
                        let core = self.core.clone();
                        let sender = sender.clone();
                        gtk::glib::spawn_future_local(async move {
                            let result = core.get_recent_sessions(None, Some(20));
                            let _ = sender.input_sender().send(AppMsg::TimelineLoaded(
                                result.map(|page| page.sessions),
                            ));
                        });
                    }
                }
            }

            AppMsg::SearchChanged(query) => {
                self.state.search_query = query;
                self.refresh_home(&sender);
            }

            AppMsg::LayoutChanged(layout) => {
                self.state.layout = layout;
                self.rebuild_list(&sender);
            }

            AppMsg::DeleteNote => {
                if let Some(id) = self.state.selected_note_id.clone() {
                    match self.core.delete_note(&id) {
                        Ok(()) => {
                            self.state.content_view = ContentView::Notes;
                            self.refresh_home(&sender);
                            self.toast_overlay.add_toast(adw::Toast::new("已删除笔记"));
                        }
                        Err(error) => {
                            self.state.status = Some(format!("删除失败: {error}"));
                        }
                    }
                }
            }

            AppMsg::SaveNote { id, content, tags } => {
                let is_edit = id.is_some();
                let result = match id {
                    Some(note_id) => self.core.edit_note(&note_id, content, tags),
                    None => self.core.create_note(content, tags),
                };

                match result {
                    Ok(note) => {
                        let view = if is_edit {
                            ContentView::NoteDetail
                        } else {
                            ContentView::Notes
                        };
                        self.state.content_view = view;
                        self.state.search_query.clear();
                        self.refresh_home_with_selection(
                            &sender,
                            note.id,
                            if is_edit {
                                "已更新笔记"
                            } else {
                                "已创建笔记"
                            },
                        );
                    }
                    Err(error) => {
                        self.state.status = Some(format!("保存失败: {error}"));
                    }
                }
            }

            AppMsg::SaveReply {
                parent_id,
                content,
                tags,
            } => {
                match self.core.reply_note(&parent_id, content, tags) {
                    Ok(_note) => {
                        self.state.content_view = ContentView::NoteDetail;
                        self.state.search_query.clear();
                        self.state.selected_note_id = Some(parent_id.clone());
                        self.state.sync_selection();
                        self.refresh_home(&sender);

                        // Reload parent note detail to show the new reply
                        let core = self.core.clone();
                        let sender = sender.clone();
                        let parent_id_for_reload = parent_id.clone();
                        gtk::glib::spawn_future_local(async move {
                            let result =
                                crate::usecase::load_note_detail(core.as_ref(), &parent_id_for_reload);
                            let _ = sender.input_sender().send(AppMsg::NoteDetailLoaded(result));
                        });

                        self.toast_overlay.add_toast(adw::Toast::new("已发送回复"));
                    }
                    Err(error) => {
                        self.state.status = Some(format!("回复失败: {error}"));
                    }
                }
            }

            AppMsg::CreateNote => {
                self.open_editor(&sender, None, false);
            }

            AppMsg::EditNote => {
                let note_id = self.state.selected_note_id.clone();
                self.open_editor(&sender, note_id, false);
            }

            AppMsg::ReplyToNote => {
                let parent_id = self.state.selected_note_id.clone();
                self.open_editor(&sender, parent_id, true);
            }

            AppMsg::ThemeChanged(theme) => {
                self.state.theme = theme;
                apply_theme(theme);
            }

            AppMsg::NoteSelected(index) => {
                let visible = self.state.visible_notes();
                if let Some(note) = visible.get(index as usize) {
                    if self.state.selected_note_id.as_deref() != Some(&note.id) {
                        self.state.selected_note_id = Some(note.id.clone());
                        self.state.sync_selection();
                    }
                }
            }

            AppMsg::NoteActivated(index) => {
                let visible = self.state.visible_notes();
                if let Some(note) = visible.get(index as usize) {
                    let note_id = note.id.clone();
                    self.state.selected_note_id = Some(note_id.clone());
                    self.state.content_view = ContentView::NoteDetail;
                    self.state.sync_selection();
                    self.rebuild_list(&sender);

                    let core = self.core.clone();
                    let sender = sender.clone();
                    gtk::glib::spawn_future_local(async move {
                        let result = crate::usecase::load_note_detail(core.as_ref(), &note_id);
                        let _ = sender.input_sender().send(AppMsg::NoteDetailLoaded(result));
                    });
                }
            }

            AppMsg::NoteDetailLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.state.selected_note_full = Some(data);
                        self.state.status = None;
                    }
                    Err(error) => {
                        self.state.status = Some(format!("加载详情失败: {error}"));
                    }
                }
            }

            AppMsg::OpenNoteDetail(note_id) => {
                self.state.selected_note_id = Some(note_id.clone());
                self.state.content_view = ContentView::NoteDetail;
                self.state.sync_selection();
                self.rebuild_list(&sender);

                let core = self.core.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let result = crate::usecase::load_note_detail(core.as_ref(), &note_id);
                    let _ = sender.input_sender().send(AppMsg::NoteDetailLoaded(result));
                });
            }

            AppMsg::LoadMoreNotes => {
                let cursor = match self.state.content_view {
                    ContentView::Notes => self.state.home.notes_cursor.clone(),
                    ContentView::Trash => self.state.home.deleted_notes_cursor.clone(),
                    _ => None,
                };

                if let Some(cursor) = cursor {
                    self.state.is_loading_more = true;
                    self.sync_ui(&sender);

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

            AppMsg::MoreNotesLoaded(result) => {
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
                        self.rebuild_list(&sender);
                    }
                    Err(error) => {
                        self.state.status = Some(format!("加载更多失败: {error}"));
                    }
                }
            }

            AppMsg::TagSelected(tag) => {
                self.state.selected_tag = Some(tag.clone());
                self.state.content_view = ContentView::TagNotes;
                self.state.sync_selection();
                self.rebuild_list(&sender);

                let core = self.core.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let result = core.get_notes_by_tag(&tag, 50);
                    let _ = sender.input_sender().send(AppMsg::TagNotesLoaded(result));
                });
            }

            AppMsg::TagsLoaded(result) => {
                match result {
                    Ok(tags) => {
                        self.state.all_tags = tags;
                        self.state.status = None;
                    }
                    Err(error) => {
                        self.state.status = Some(format!("加载标签失败: {error}"));
                    }
                }
            }

            AppMsg::TagNotesLoaded(result) => {
                match result {
                    Ok(notes) => {
                        self.state.tag_notes = notes;
                        self.state.sync_selection();
                        self.rebuild_list(&sender);
                        self.state.status = None;
                    }
                    Err(error) => {
                        self.state.status = Some(format!("加载标签笔记失败: {error}"));
                    }
                }
            }

            AppMsg::TagSuggestionsLoaded(result) => {
                match result {
                    Ok(suggestions) => {
                        self.state.tag_suggestions = suggestions;
                    }
                    Err(_error) => {
                        // Silently ignore tag suggestion errors
                    }
                }
            }

            AppMsg::TimelineLoaded(result) => {
                match result {
                    Ok(sessions) => {
                        self.state.timeline_sessions = sessions;
                        self.state.status = None;
                    }
                    Err(error) => {
                        self.state.status = Some(format!("加载时间线失败: {error}"));
                    }
                }
            }

            AppMsg::ClearFilters => {
                self.state.selected_tag = None;
                self.state.search_query.clear();
                self.state.content_view = ContentView::Notes;
                self.state.tag_notes.clear();
                self.state.sync_selection();
                self.refresh_home(&sender);
            }
        }
        self.sync_ui(&sender);
    }
}

impl App {
    fn rebuild_list(&self, sender: &ComponentSender<Self>) {
        // Clear list layout
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Clear waterfall layout
        let flow_box = self.get_flow_box();
        while let Some(child) = flow_box.first_child() {
            flow_box.remove(&child);
        }

        let visible = self.state.visible_notes();

        // Populate list layout
        for note in &visible {
            let row = build_note_row(note);
            self.list_box.append(&row);
        }

        // Populate waterfall layout
        for note in &visible {
            let card = build_waterfall_card(note, sender);
            flow_box.append(&card);
        }

        // Show loading indicator at bottom when loading more
        if self.state.is_loading_more {
            let loading_row = gtk::ListBoxRow::new();
            let spinner = gtk::Spinner::new();
            spinner.set_margin_top(12);
            spinner.set_margin_bottom(12);
            spinner.start();
            loading_row.set_child(Some(&spinner));
            loading_row.set_activatable(false);
            self.list_box.append(&loading_row);
        }

        if let Some(index) = self.state.selected_index_in(&visible) {
            if let Some(row) = self.list_box.row_at_index(index as i32) {
                self.list_box.select_row(Some(&row));
            }
        }
    }

    fn get_flow_box(&self) -> &gtk::FlowBox {
        &self.flow_box
    }

    fn sync_ui(&self, sender: &ComponentSender<Self>) {
        let is_empty = self.state.visible_notes().is_empty();
        let child_name = match self.state.content_view {
            ContentView::NoteDetail => "detail",
            ContentView::Settings => "settings",
            ContentView::Tags => "tags",
            ContentView::Timeline => "timeline",
            ContentView::TagNotes => {
                if is_empty {
                    "empty"
                } else {
                    "notes"
                }
            }
            _ => {
                if is_empty {
                    "empty"
                } else {
                    "notes"
                }
            }
        };
        self.content_stack.set_visible_child_name(child_name);

        // Switch between list and waterfall layouts
        if matches!(
            self.state.content_view,
            ContentView::Notes | ContentView::Trash | ContentView::TagNotes
        ) {
            let layout_name = match self.state.layout {
                NoteLayout::List => "list",
                NoteLayout::Waterfall => "waterfall",
            };
            self.layout_stack.set_visible_child_name(layout_name);
        }

        // Update empty page
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
            ContentView::Timeline => ("", String::new()),
            _ => ("", String::new()),
        };
        self.empty_page.set_title(title);
        self.empty_page.set_description(Some(&desc));

        // Update detail rows
        self.detail_content_row.set_subtitle(&self.detail_content());
        self.detail_tags_row.set_subtitle(&self.detail_tags());
        self.detail_meta_row.set_subtitle(&self.detail_meta());

        // Update origins - remove all but the label
        while self.detail_origins_box.observe_children().n_items() > 1 {
            if let Some(child) = self.detail_origins_box.last_child() {
                self.detail_origins_box.remove(&child);
            }
        }
        if let Some(full) = &self.state.selected_note_full {
            if full.origins.is_empty() {
                let row = adw::ActionRow::builder().title("无溯源").build();
                self.detail_origins_box.append(&row);
            } else {
                for origin in &full.origins {
                    let note_id = origin.id.clone();
                    let row = build_clickable_note_row(origin, sender, note_id);
                    self.detail_origins_box.append(&row);
                }
            }
        }

        // Update replies - remove all but the label
        while self.detail_replies_box.observe_children().n_items() > 1 {
            if let Some(child) = self.detail_replies_box.last_child() {
                self.detail_replies_box.remove(&child);
            }
        }
        if let Some(full) = &self.state.selected_note_full {
            if full.replies.is_empty() {
                let row = adw::ActionRow::builder().title("无回复").build();
                self.detail_replies_box.append(&row);
            } else {
                for reply in &full.replies {
                    let note_id = reply.id.clone();
                    let row = build_clickable_note_row(reply, sender, note_id);
                    self.detail_replies_box.append(&row);
                }
            }
        }

        // Update versions - remove all but the label
        while self.detail_versions_box.observe_children().n_items() > 1 {
            if let Some(child) = self.detail_versions_box.last_child() {
                self.detail_versions_box.remove(&child);
            }
        }
        if let Some(full) = &self.state.selected_note_full {
            if full.other_versions.is_empty() {
                let row = adw::ActionRow::builder().title("无其他版本").build();
                self.detail_versions_box.append(&row);
            } else {
                for version in &full.other_versions {
                    let note_id = version.id.clone();
                    let row = build_clickable_note_row(version, sender, note_id);
                    self.detail_versions_box.append(&row);
                }
            }
        }

        // Update theme dropdown without triggering signal
        let theme_idx = self.state.theme.index();
        if self.theme_dropdown.selected() != theme_idx {
            self.theme_dropdown.set_selected(theme_idx);
        }

        // Update tags page
        while let Some(child) = self.tags_flow_box.first_child() {
            self.tags_flow_box.remove(&child);
        }
        for tag in &self.state.all_tags {
            let button = gtk::Button::with_label(&format!("#{tag}"));
            button.add_css_class("pill");
            let tag_clone = tag.clone();
            let sender_clone = sender.input_sender().clone();
            button.connect_clicked(move |_| {
                let _ = sender_clone.send(AppMsg::TagSelected(tag_clone.clone()));
            });
            self.tags_flow_box.append(&button);
        }

        // Update timeline page
        while let Some(child) = self.timeline_container.first_child() {
            self.timeline_container.remove(&child);
        }
        for session in &self.state.timeline_sessions {
            let session_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
            
            let start_time = crate::domain::format_timestamp(session.started_at);
            let end_time = crate::domain::format_timestamp(session.ended_at);
            let header = gtk::Label::new(Some(&format!(
                "{} - {} · {} 条笔记",
                start_time, end_time, session.note_count
            )));
            header.add_css_class("heading");
            header.set_halign(gtk::Align::Start);
            session_box.append(&header);
            
            let notes_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
            for note in &session.notes {
                let note_id = note.id.clone();
                let row = build_clickable_note_row(note, sender, note_id);
                notes_box.append(&row);
            }
            session_box.append(&notes_box);
            
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.set_margin_top(12);
            separator.set_margin_bottom(12);
            session_box.append(&separator);
            
            self.timeline_container.append(&session_box);
        }
    }

    fn refresh_home(&mut self, sender: &ComponentSender<Self>) {
        let query = self.state.search_query.clone();
        match load_home(self.core.as_ref(), &query) {
            Ok(home) => {
                self.state.home = home;
                self.state.sync_selection();
                self.state.status = None;
            }
            Err(error) => {
                self.state.status = Some(format!("加载失败: {error}"));
            }
        }
        self.rebuild_list(sender);
    }

    fn refresh_home_with_selection(&mut self, sender: &ComponentSender<Self>, note_id: String, toast_msg: &str) {
        let query = self.state.search_query.clone();
        match load_home(self.core.as_ref(), &query) {
            Ok(home) => {
                self.state.home = home;
                self.state.selected_note_id = Some(note_id);
                self.state.sync_selection();
                self.state.status = None;
            }
            Err(error) => {
                self.state.status = Some(format!("加载失败: {error}"));
            }
        }
        self.rebuild_list(sender);
        self.toast_overlay.add_toast(adw::Toast::new(toast_msg));
    }

    fn open_editor(&self, sender: &ComponentSender<Self>, note_id: Option<String>, is_reply: bool) {
        let (title, _hint, initial_content, initial_tags) = if is_reply {
            (
                "回复笔记",
                "写下你的回复内容；标签可选，使用逗号分隔。",
                String::new(),
                Vec::new(),
            )
        } else if let Some(_id) = &note_id {
            let detail = self.state.selected_note_detail.clone();
            if let Some(d) = detail {
                (
                    "编辑笔记",
                    "修改正文或标签，保存后会立即更新详情。",
                    d.content,
                    d.tags,
                )
            } else {
                return;
            }
        } else {
            (
                "新建笔记",
                "直接记录内容；标签可选，使用逗号分隔。",
                String::new(),
                Vec::new(),
            )
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

    fn detail_content(&self) -> String {
        self.state
            .selected_note_detail
            .as_ref()
            .map(|d| d.content.clone())
            .unwrap_or_else(|| "请先从列表中打开一条笔记。".to_string())
    }

    fn detail_tags(&self) -> String {
        self.state
            .selected_note_detail
            .as_ref()
            .map(|d| {
                if d.tags.is_empty() {
                    "暂无标签".to_string()
                } else {
                    d.tags
                        .iter()
                        .map(|t| format!("#{t}"))
                        .collect::<Vec<_>>()
                        .join("  ")
                }
            })
            .unwrap_or_default()
    }

    fn detail_meta(&self) -> String {
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

fn build_note_row(note: &synap_core::dto::NoteDTO) -> gtk::ListBoxRow {
    let preview = crate::domain::NoteListItemViewModel::from(note).preview;

    let action_row = adw::ActionRow::new();
    action_row.set_title(&preview);
    action_row.set_subtitle(
        &note
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join("  "),
    );
    action_row.set_activatable(true);

    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&action_row));
    row
}

fn build_clickable_note_row(
    note: &synap_core::dto::NoteDTO,
    sender: &ComponentSender<App>,
    note_id: String,
) -> adw::ActionRow {
    let preview = crate::domain::NoteListItemViewModel::from(note).preview;

    let action_row = adw::ActionRow::new();
    action_row.set_title(&preview);
    action_row.set_subtitle(
        &note
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join("  "),
    );
    action_row.set_activatable(true);

    let gesture = gtk::GestureClick::new();
    let sender_clone = sender.input_sender().clone();
    gesture.connect_released(move |_, _, _, _| {
        let _ = sender_clone.send(AppMsg::OpenNoteDetail(note_id.clone()));
    });
    action_row.add_controller(gesture);

    action_row
}

fn build_waterfall_card(
    note: &synap_core::dto::NoteDTO,
    sender: &ComponentSender<App>,
) -> gtk::FlowBoxChild {
    let preview = crate::domain::NoteListItemViewModel::from(note).preview;

    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.set_margin_top(12);
    card.set_margin_bottom(12);
    card.set_margin_start(12);
    card.set_margin_end(12);
    card.set_hexpand(true);

    // Content preview
    let content_label = gtk::Label::new(Some(&preview));
    content_label.set_wrap(true);
    content_label.set_max_width_chars(15);
    content_label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    content_label.set_justify(gtk::Justification::Left);
    content_label.set_halign(gtk::Align::Start);
    content_label.set_xalign(0.0);
    content_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    card.append(&content_label);

    // Tags
    if !note.tags.is_empty() {
        let tags_label = gtk::Label::new(Some(
            &note.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join("  ")
        ));
        tags_label.add_css_class("caption");
        tags_label.set_halign(gtk::Align::Start);
        tags_label.set_xalign(0.0);
        tags_label.set_wrap(true);
        tags_label.set_max_width_chars(15);
        card.append(&tags_label);
    }

    // Time
    let time_label = gtk::Label::new(Some(&crate::domain::format_timestamp(note.created_at)));
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

fn apply_theme(theme: Theme) {
    let manager = adw::StyleManager::default();
    match theme {
        Theme::Auto => manager.set_color_scheme(adw::ColorScheme::Default),
        Theme::Light => manager.set_color_scheme(adw::ColorScheme::ForceLight),
        Theme::Dark => manager.set_color_scheme(adw::ColorScheme::ForceDark),
    }
}
