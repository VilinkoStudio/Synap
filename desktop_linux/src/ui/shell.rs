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
    pub detail_content_row: adw::ActionRow,
    pub detail_tags_row: adw::ActionRow,
    pub detail_meta_row: adw::ActionRow,
    pub detail_origins_box: gtk::Box,
    pub detail_replies_box: gtk::Box,
    pub detail_versions_box: gtk::Box,
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

    let (layout_stack, flow_box, notes_page) = build_notes_page(&list_box, sender);
    content_stack.add_named(&notes_page, Some("notes"));

    let empty_page = adw::StatusPage::new();
    empty_page.set_icon_name(Some("network-workgroup-symbolic"));
    content_stack.add_named(&empty_page, Some("empty"));

    let detail_page = build_detail_page(sender);
    content_stack.add_named(&detail_page.root, Some("detail"));

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
        detail_content_row: detail_page.content_row,
        detail_tags_row: detail_page.tags_row,
        detail_meta_row: detail_page.meta_row,
        detail_origins_box: detail_page.origins_box,
        detail_replies_box: detail_page.replies_box,
        detail_versions_box: detail_page.versions_box,
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

pub fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_notes_page(
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

struct DetailPage {
    root: gtk::ScrolledWindow,
    content_row: adw::ActionRow,
    tags_row: adw::ActionRow,
    meta_row: adw::ActionRow,
    origins_box: gtk::Box,
    replies_box: gtk::Box,
    versions_box: gtk::Box,
}

fn build_detail_page(sender: &ComponentSender<App>) -> DetailPage {
    let detail_clamp = adw::Clamp::builder()
        .maximum_size(800)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let detail_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
    let detail_group = adw::PreferencesGroup::builder().title("笔记内容").build();

    let content_row = adw::ActionRow::builder()
        .title("内容")
        .subtitle_selectable(true)
        .build();
    let tags_row = adw::ActionRow::builder().title("标签").build();
    let meta_row = adw::ActionRow::builder().title("创建时间").build();

    detail_group.add(&content_row);
    detail_group.add(&tags_row);
    detail_group.add(&meta_row);

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

    let origins_box = build_detail_section("溯源链");
    let replies_box = build_detail_section("回复");
    let versions_box = build_detail_section("其他版本");

    detail_box.append(&detail_group);
    detail_box.append(&origins_box);
    detail_box.append(&replies_box);
    detail_box.append(&versions_box);
    detail_box.append(&detail_buttons);
    detail_clamp.set_child(Some(&detail_box));

    let root = gtk::ScrolledWindow::new();
    root.set_child(Some(&detail_clamp));

    DetailPage {
        root,
        content_row,
        tags_row,
        meta_row,
        origins_box,
        replies_box,
        versions_box,
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
        ContentView::NoteDetail => "detail",
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
