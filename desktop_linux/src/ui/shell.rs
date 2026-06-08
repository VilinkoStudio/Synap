use adw::prelude::*;
use relm4::prelude::*;

use crate::{
    app::{App, message::AppMsg},
    domain::{AppState, ContentView, Theme},
};

pub struct ContentPages {
    pub content_stack: gtk::Stack,
    pub list_box: gtk::ListBox,
    pub empty_page: adw::StatusPage,
    pub workspace_stack: gtk::Stack,
    pub detail_content_label: gtk::Label,
    pub detail_tags_box: gtk::Box,
    pub detail_meta_label: gtk::Label,
    pub detail_toolbar: gtk::Box,
    pub context_toggle: gtk::ToggleButton,
    pub context_panel: gtk::ScrolledWindow,
    pub detail_origins_box: gtk::Box,
    pub detail_replies_box: gtk::Box,
    pub detail_versions_box: gtk::Box,
    pub draft_title_label: gtk::Label,
    pub draft_hint_label: gtk::Label,
    pub draft_content_buffer: gtk::TextBuffer,
    pub draft_content_view: gtk::TextView,
    pub draft_tags_entry: gtk::Entry,
    pub theme_dropdown: gtk::DropDown,
    pub sync_listener_row: adw::ActionRow,
    pub sync_addresses_row: adw::ActionRow,
    pub sync_identity_row: adw::ActionRow,
    pub sync_signing_row: adw::ActionRow,
    pub sync_error_label: gtk::Label,
    pub sync_host_entry: gtk::Entry,
    pub sync_port_entry: gtk::Entry,
    pub sync_discovered_box: gtk::Box,
    pub sync_connections_box: gtk::Box,
    pub sync_peers_box: gtk::Box,
    pub sync_sessions_box: gtk::Box,
    pub tags_flow_box: gtk::FlowBox,
    pub timeline_container: gtk::Box,
    pub layout_stack: gtk::Stack,
    pub flow_box: gtk::FlowBox,
}

pub fn build_content_pages(state: &AppState, sender: &ComponentSender<App>) -> ContentPages {
    let content_stack = gtk::Stack::new();
    content_stack.set_hexpand(true);
    content_stack.set_vexpand(true);
    content_stack.set_margin_start(12);
    content_stack.set_margin_end(12);
    content_stack.set_margin_top(12);
    content_stack.set_margin_bottom(12);

    let list_box = gtk::ListBox::new();
    list_box.set_css_classes(&["boxed-list"]);
    list_box.set_selection_mode(gtk::SelectionMode::Single);
    list_box.set_vexpand(true);

    let (layout_stack, flow_box, notes_browser) = build_notes_browser(&list_box, sender);
    let detail_page = build_detail_page(sender);
    let draft_page = build_draft_page(sender);
    let workspace_stack = gtk::Stack::new();
    workspace_stack.set_hexpand(true);
    workspace_stack.set_vexpand(true);
    workspace_stack.add_named(&detail_page.article_scroller, Some("read"));
    workspace_stack.add_named(&draft_page.root, Some("draft"));
    let notes_page =
        build_notes_workbench(&notes_browser, &workspace_stack, &detail_page.context_panel);
    content_stack.add_named(&notes_page, Some("notes"));

    let empty_page = adw::StatusPage::new();
    empty_page.set_icon_name(Some("network-workgroup-symbolic"));
    content_stack.add_named(&empty_page, Some("empty"));

    let settings_page = build_settings_page(state, sender);
    content_stack.add_named(&settings_page.root, Some("settings"));

    let tags_page = build_tags_page();
    content_stack.add_named(&tags_page.root, Some("tags"));

    let timeline_page = build_timeline_page();
    content_stack.add_named(&timeline_page.root, Some("timeline"));

    let initial_child = initial_content_child(state);
    content_stack.set_visible_child_name(initial_child);

    ContentPages {
        content_stack,
        list_box,
        empty_page,
        workspace_stack,
        detail_content_label: detail_page.content_label,
        detail_tags_box: detail_page.tags_box,
        detail_meta_label: detail_page.meta_label,
        detail_toolbar: detail_page.toolbar,
        context_toggle: detail_page.context_toggle,
        context_panel: detail_page.context_panel,
        detail_origins_box: detail_page.origins_box,
        detail_replies_box: detail_page.replies_box,
        detail_versions_box: detail_page.versions_box,
        draft_title_label: draft_page.title_label,
        draft_hint_label: draft_page.hint_label,
        draft_content_buffer: draft_page.content_buffer,
        draft_content_view: draft_page.content_view,
        draft_tags_entry: draft_page.tags_entry,
        theme_dropdown: settings_page.theme_dropdown,
        sync_listener_row: settings_page.sync_listener_row,
        sync_addresses_row: settings_page.sync_addresses_row,
        sync_identity_row: settings_page.sync_identity_row,
        sync_signing_row: settings_page.sync_signing_row,
        sync_error_label: settings_page.sync_error_label,
        sync_host_entry: settings_page.sync_host_entry,
        sync_port_entry: settings_page.sync_port_entry,
        sync_discovered_box: settings_page.sync_discovered_box,
        sync_connections_box: settings_page.sync_connections_box,
        sync_peers_box: settings_page.sync_peers_box,
        sync_sessions_box: settings_page.sync_sessions_box,
        tags_flow_box: tags_page.tags_flow_box,
        timeline_container: timeline_page.timeline_container,
        layout_stack,
        flow_box,
    }
}

fn build_notes_browser(
    list_box: &gtk::ListBox,
    sender: &ComponentSender<App>,
) -> (gtk::Stack, gtk::FlowBox, gtk::ScrolledWindow) {
    let layout_stack = gtk::Stack::new();

    let list_scroller = gtk::ScrolledWindow::new();
    list_scroller.set_child(Some(list_box));
    layout_stack.add_named(&list_scroller, Some("list"));

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

    let notes_scroller = gtk::ScrolledWindow::new();
    notes_scroller.set_child(Some(&layout_stack));

    let sender_scroll = sender.input_sender().clone();
    notes_scroller
        .vadjustment()
        .connect_value_changed(move |adj| {
            let upper = adj.upper();
            let page_size = adj.page_size();
            let value = adj.value();

            if upper > page_size && value >= upper - page_size - 100.0 {
                let _ = sender_scroll.send(AppMsg::LoadMoreNotes);
            }
        });

    (layout_stack, flow_box, notes_scroller)
}

fn build_notes_workbench(
    notes_browser: &gtk::ScrolledWindow,
    workspace_stack: &gtk::Stack,
    context_panel: &gtk::ScrolledWindow,
) -> gtk::Paned {
    let root = gtk::Paned::new(gtk::Orientation::Horizontal);
    root.set_wide_handle(false);
    root.set_position(340);
    root.set_shrink_start_child(false);
    root.set_shrink_end_child(false);

    notes_browser.set_min_content_width(280);
    notes_browser.set_max_content_width(440);

    let writing_area = gtk::Paned::new(gtk::Orientation::Horizontal);
    writing_area.set_wide_handle(true);
    writing_area.set_position(760);
    writing_area.set_shrink_start_child(false);
    writing_area.set_shrink_end_child(false);
    writing_area.set_start_child(Some(workspace_stack));
    writing_area.set_end_child(Some(context_panel));

    root.set_start_child(Some(notes_browser));
    root.set_end_child(Some(&writing_area));
    root
}

struct DetailPage {
    article_scroller: gtk::ScrolledWindow,
    context_panel: gtk::ScrolledWindow,
    content_label: gtk::Label,
    tags_box: gtk::Box,
    meta_label: gtk::Label,
    toolbar: gtk::Box,
    context_toggle: gtk::ToggleButton,
    origins_box: gtk::Box,
    replies_box: gtk::Box,
    versions_box: gtk::Box,
}

fn build_detail_page(sender: &ComponentSender<App>) -> DetailPage {
    let article_scroller = gtk::ScrolledWindow::new();
    article_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);

    let detail_clamp = adw::Clamp::builder()
        .maximum_size(760)
        .margin_top(36)
        .margin_bottom(48)
        .margin_start(32)
        .margin_end(32)
        .build();

    let article = gtk::Box::new(gtk::Orientation::Vertical, 22);
    article.set_margin_top(12);
    article.set_margin_bottom(12);
    article.set_margin_start(12);
    article.set_margin_end(12);

    let meta_label = gtk::Label::new(None);
    meta_label.add_css_class("caption");
    meta_label.add_css_class("dim-label");
    meta_label.set_halign(gtk::Align::Start);
    meta_label.set_xalign(0.0);
    meta_label.set_wrap(true);

    let content_label = gtk::Label::new(None);
    content_label.add_css_class("title-4");
    content_label.set_halign(gtk::Align::Fill);
    content_label.set_xalign(0.0);
    content_label.set_wrap(true);
    content_label.set_selectable(true);
    content_label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    content_label.set_justify(gtk::Justification::Left);

    let tags_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    tags_box.set_halign(gtk::Align::Start);
    tags_box.set_hexpand(true);

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    toolbar.set_halign(gtk::Align::End);

    let context_toggle = gtk::ToggleButton::new();
    context_toggle.set_icon_name("view-sidebar-end-symbolic");
    context_toggle.set_tooltip_text(Some("显示或隐藏上下文"));
    let sender_context = sender.input_sender().clone();
    context_toggle.connect_clicked(move |_| {
        let _ = sender_context.send(AppMsg::ToggleContextPanel);
    });

    let reply_button = gtk::Button::with_label("回复笔记");
    reply_button.set_icon_name("mail-reply-sender-symbolic");
    let sender_reply = sender.input_sender().clone();
    reply_button.connect_clicked(move |_| {
        let _ = sender_reply.send(AppMsg::ReplyToNote);
    });

    let edit_button = gtk::Button::with_label("编辑笔记");
    edit_button.set_icon_name("document-edit-symbolic");
    edit_button.add_css_class("suggested-action");
    let sender_edit = sender.input_sender().clone();
    edit_button.connect_clicked(move |_| {
        let _ = sender_edit.send(AppMsg::EditNote);
    });

    let delete_button = gtk::Button::with_label("删除笔记");
    delete_button.set_icon_name("user-trash-symbolic");
    delete_button.add_css_class("destructive-action");
    let sender_delete = sender.input_sender().clone();
    delete_button.connect_clicked(move |_| {
        let _ = sender_delete.send(AppMsg::DeleteNote);
    });

    toolbar.append(&context_toggle);
    toolbar.append(&reply_button);
    toolbar.append(&edit_button);
    toolbar.append(&delete_button);

    article.append(&meta_label);
    article.append(&content_label);
    article.append(&tags_box);
    article.append(&toolbar);
    detail_clamp.set_child(Some(&article));
    article_scroller.set_child(Some(&detail_clamp));

    let context_scroller = gtk::ScrolledWindow::new();
    context_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    context_scroller.set_min_content_width(300);
    context_scroller.set_max_content_width(420);

    let context_box = gtk::Box::new(gtk::Orientation::Vertical, 18);
    context_box.set_margin_top(24);
    context_box.set_margin_bottom(24);
    context_box.set_margin_start(18);
    context_box.set_margin_end(18);

    let context_title = gtk::Label::new(Some("Synap 上下文"));
    context_title.add_css_class("title-4");
    context_title.set_halign(gtk::Align::Start);
    context_title.set_xalign(0.0);
    context_box.append(&context_title);

    let context_desc = gtk::Label::new(Some(
        "这里展示思路的 DAG 连接、回复延展和版本演化；正文区域只保留沉浸式阅读与书写。",
    ));
    context_desc.add_css_class("caption");
    context_desc.add_css_class("dim-label");
    context_desc.set_wrap(true);
    context_desc.set_xalign(0.0);
    context_box.append(&context_desc);

    let origins_box = build_detail_section("溯源链");
    let replies_box = build_detail_section("回复");
    let versions_box = build_detail_section("版本演化");
    context_box.append(&origins_box);
    context_box.append(&replies_box);
    context_box.append(&versions_box);
    context_scroller.set_child(Some(&context_box));

    DetailPage {
        article_scroller,
        context_panel: context_scroller,
        content_label,
        tags_box,
        meta_label,
        toolbar,
        context_toggle,
        origins_box,
        replies_box,
        versions_box,
    }
}

struct DraftPage {
    root: gtk::ScrolledWindow,
    title_label: gtk::Label,
    hint_label: gtk::Label,
    content_buffer: gtk::TextBuffer,
    content_view: gtk::TextView,
    tags_entry: gtk::Entry,
}

fn build_draft_page(sender: &ComponentSender<App>) -> DraftPage {
    let root = gtk::ScrolledWindow::new();
    root.set_hscrollbar_policy(gtk::PolicyType::Never);

    let clamp = adw::Clamp::builder()
        .maximum_size(780)
        .margin_top(28)
        .margin_bottom(36)
        .margin_start(32)
        .margin_end(32)
        .build();

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 14);
    shell.set_margin_top(12);
    shell.set_margin_bottom(12);
    shell.set_margin_start(12);
    shell.set_margin_end(12);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_hexpand(true);

    let title_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    title_box.set_hexpand(true);
    let title_label = gtk::Label::new(None);
    title_label.add_css_class("title-3");
    title_label.set_halign(gtk::Align::Start);
    title_label.set_xalign(0.0);
    let hint_label = gtk::Label::new(None);
    hint_label.add_css_class("caption");
    hint_label.add_css_class("dim-label");
    hint_label.set_halign(gtk::Align::Start);
    hint_label.set_xalign(0.0);
    hint_label.set_wrap(true);
    title_box.append(&title_label);
    title_box.append(&hint_label);

    let cancel_button = gtk::Button::with_label("取消");
    let sender_cancel = sender.input_sender().clone();
    cancel_button.connect_clicked(move |_| {
        let _ = sender_cancel.send(AppMsg::CancelDraft);
    });

    let save_button = gtk::Button::with_label("保存");
    save_button.add_css_class("suggested-action");
    let sender_save = sender.input_sender().clone();
    save_button.connect_clicked(move |_| {
        let _ = sender_save.send(AppMsg::SaveDraft);
    });

    header.append(&title_box);
    header.append(&cancel_button);
    header.append(&save_button);

    let content_buffer = gtk::TextBuffer::new(None);
    let content_view = gtk::TextView::with_buffer(&content_buffer);
    content_view.set_vexpand(true);
    content_view.set_wrap_mode(gtk::WrapMode::WordChar);
    content_view.set_top_margin(22);
    content_view.set_bottom_margin(22);
    content_view.set_left_margin(22);
    content_view.set_right_margin(22);
    content_view.set_monospace(true);

    let sender_content = sender.input_sender().clone();
    content_buffer.connect_changed(move |buffer| {
        let (start, end) = buffer.bounds();
        let text = buffer.text(&start, &end, false).to_string();
        let _ = sender_content.send(AppMsg::DraftContentChanged(text));
    });

    let content_frame = gtk::ScrolledWindow::new();
    content_frame.set_vexpand(true);
    content_frame.set_min_content_height(420);
    content_frame.set_child(Some(&content_view));

    let tags_entry = gtk::Entry::new();
    tags_entry.set_placeholder_text(Some("标签，例如 rust, idea, diary"));
    let sender_tags = sender.input_sender().clone();
    tags_entry.connect_changed(move |entry| {
        let _ = sender_tags.send(AppMsg::DraftTagsChanged(entry.text().to_string()));
    });

    shell.append(&header);
    shell.append(&content_frame);
    shell.append(&tags_entry);
    clamp.set_child(Some(&shell));
    root.set_child(Some(&clamp));

    DraftPage {
        root,
        title_label,
        hint_label,
        content_buffer,
        content_view,
        tags_entry,
    }
}

struct SettingsPage {
    root: gtk::ScrolledWindow,
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
}

fn build_settings_page(state: &AppState, sender: &ComponentSender<App>) -> SettingsPage {
    let page = adw::PreferencesPage::new();
    let root = gtk::ScrolledWindow::new();
    root.set_child(Some(&page));

    let sync_group = adw::PreferencesGroup::builder()
        .title("同步与信任")
        .description("桌面端监听、局域网发现、手动连接、设备信任和同步历史")
        .build();

    let sync_listener_row = adw::ActionRow::builder()
        .title("监听状态")
        .subtitle("正在读取…")
        .build();
    let refresh_button = gtk::Button::with_label("刷新");
    let refresh_sender = sender.input_sender().clone();
    refresh_button.connect_clicked(move |_| {
        let _ = refresh_sender.send(AppMsg::RefreshSync);
    });
    sync_listener_row.add_suffix(&refresh_button);
    sync_group.add(&sync_listener_row);

    let sync_addresses_row = adw::ActionRow::builder()
        .title("局域网地址")
        .subtitle("—")
        .build();
    sync_group.add(&sync_addresses_row);

    let sync_identity_row = adw::ActionRow::builder()
        .title("身份公钥")
        .subtitle("—")
        .build();
    sync_group.add(&sync_identity_row);

    let sync_signing_row = adw::ActionRow::builder()
        .title("签名公钥")
        .subtitle("—")
        .build();
    sync_group.add(&sync_signing_row);

    let sync_error_label = gtk::Label::new(None);
    sync_error_label.add_css_class("error");
    sync_error_label.set_halign(gtk::Align::Start);
    sync_error_label.set_wrap(true);
    sync_error_label.set_margin_start(12);
    sync_error_label.set_margin_end(12);
    sync_group.add(&sync_error_label);
    page.add(&sync_group);

    let connections_group = adw::PreferencesGroup::builder()
        .title("连接目标")
        .description("自动发现的设备可直接配对，也支持手动添加主机与端口")
        .build();

    let add_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    add_box.set_margin_top(6);
    add_box.set_margin_bottom(6);
    add_box.set_margin_start(12);
    add_box.set_margin_end(12);

    let sync_host_entry = gtk::Entry::new();
    sync_host_entry.set_hexpand(true);
    sync_host_entry.set_placeholder_text(Some("主机地址"));
    sync_host_entry.set_text(&state.sync.host_input);
    let host_sender = sender.input_sender().clone();
    sync_host_entry.connect_changed(move |entry| {
        let _ = host_sender.send(AppMsg::UpdateSyncHost(entry.text().to_string()));
    });

    let sync_port_entry = gtk::Entry::new();
    sync_port_entry.set_width_chars(8);
    sync_port_entry.set_placeholder_text(Some("端口"));
    sync_port_entry.set_input_purpose(gtk::InputPurpose::Digits);
    sync_port_entry.set_text(&state.sync.port_input);
    let port_sender = sender.input_sender().clone();
    sync_port_entry.connect_changed(move |entry| {
        let _ = port_sender.send(AppMsg::UpdateSyncPort(entry.text().to_string()));
    });

    let add_button = gtk::Button::with_label("添加");
    add_button.add_css_class("suggested-action");
    let add_sender = sender.input_sender().clone();
    add_button.connect_clicked(move |_| {
        let _ = add_sender.send(AppMsg::AddSyncConnection);
    });

    add_box.append(&sync_host_entry);
    add_box.append(&sync_port_entry);
    add_box.append(&add_button);
    connections_group.add(&add_box);

    let sync_discovered_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let sync_connections_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    connections_group.add(&section_label("局域网发现"));
    connections_group.add(&sync_discovered_box);
    connections_group.add(&section_label("已保存连接"));
    connections_group.add(&sync_connections_box);
    page.add(&connections_group);

    let peers_group = adw::PreferencesGroup::builder()
        .title("设备列表")
        .description("管理待信任、已信任和已撤销的对端公钥")
        .build();
    let sync_peers_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    peers_group.add(&sync_peers_box);
    page.add(&peers_group);

    let sessions_group = adw::PreferencesGroup::builder()
        .title("同步统计")
        .description("最近同步结果和角色信息")
        .build();
    let sync_sessions_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sessions_group.add(&sync_sessions_box);

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
    page.add(&sessions_group);
    page.add(&settings_group);

    SettingsPage {
        root,
        theme_dropdown,
        sync_listener_row,
        sync_addresses_row,
        sync_identity_row,
        sync_signing_row,
        sync_error_label,
        sync_host_entry,
        sync_port_entry,
        sync_discovered_box,
        sync_connections_box,
        sync_peers_box,
        sync_sessions_box,
    }
}

struct TagsPage {
    root: gtk::ScrolledWindow,
    tags_flow_box: gtk::FlowBox,
}

fn build_tags_page() -> TagsPage {
    let root = gtk::ScrolledWindow::new();
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

    tags_box.append(&tags_flow_box);
    root.set_child(Some(&tags_box));

    TagsPage {
        root,
        tags_flow_box,
    }
}

struct TimelinePage {
    root: gtk::ScrolledWindow,
    timeline_container: gtk::Box,
}

fn build_timeline_page() -> TimelinePage {
    let root = gtk::ScrolledWindow::new();
    let timeline_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
    timeline_box.set_margin_top(24);
    timeline_box.set_margin_bottom(24);
    timeline_box.set_margin_start(24);
    timeline_box.set_margin_end(24);

    let timeline_container = gtk::Box::new(gtk::Orientation::Vertical, 24);
    timeline_box.append(&timeline_container);

    root.set_child(Some(&timeline_box));

    TimelinePage {
        root,
        timeline_container,
    }
}

fn build_detail_section(title: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let label = gtk::Label::new(Some(title));
    label.add_css_class("heading");
    label.set_halign(gtk::Align::Start);
    section.append(&label);
    section
}

fn initial_content_child(state: &AppState) -> &'static str {
    let is_empty = state.visible_notes().is_empty();
    match state.content_view {
        ContentView::NoteDetail => "notes",
        ContentView::Settings => "settings",
        ContentView::Tags => "tags",
        ContentView::Timeline => "timeline",
        ContentView::TagNotes | ContentView::Notes | ContentView::Trash => {
            if is_empty {
                "empty"
            } else {
                "notes"
            }
        }
    }
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("heading");
    label.set_halign(gtk::Align::Start);
    label.set_margin_top(12);
    label.set_margin_start(12);
    label
}
