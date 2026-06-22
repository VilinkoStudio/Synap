use adw::prelude::*;
use relm4::prelude::*;

use crate::{
    app::{App, message::AppMsg},
    domain::{AppState, ContentView, Theme},
    ui::editor::WysiwygEditor,
};

/// Configure a ScrolledWindow with smooth scrolling defaults.
fn smooth_scroller(sw: &gtk::ScrolledWindow) {
    sw.set_kinetic_scrolling(true);
    sw.set_overlay_scrolling(true);
    sw.set_propagate_natural_height(true);
}

pub struct ContentPages {
    pub content_stack: gtk::Stack,
    pub list_box: gtk::ListBox,
    pub empty_page: adw::StatusPage,

    // reading/editing page (shared editor)
    pub reading_context_panel: gtk::ScrolledWindow,
    pub reading_editor: WysiwygEditor,
    pub reading_tags_box: gtk::Box,
    pub reading_meta_label: gtk::Label,
    pub reading_origins_box: gtk::Box,
    pub reading_replies_box: gtk::Box,
    pub reading_versions_box: gtk::Box,

    // editing overlay (tags entry, shown only in edit mode)
    pub editing_title_label: gtk::Label,
    pub editing_hint_label: gtk::Label,
    pub editing_tags_entry: gtk::Entry,

    // settings
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

    // tags
    pub tags_flow_box: gtk::FlowBox,

    // timeline
    pub timeline_container: gtk::Box,
}

pub fn build_content_pages(state: &AppState, sender: &ComponentSender<App>) -> ContentPages {
    let content_stack = gtk::Stack::new();
    content_stack.set_hexpand(true);
    content_stack.set_vexpand(true);
    content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    content_stack.set_transition_duration(180);

    // ── Browse pages ──

    let list_box = gtk::ListBox::new();
    list_box.set_css_classes(&["boxed-list"]);
    list_box.set_selection_mode(gtk::SelectionMode::Single);
    list_box.set_vexpand(true);

    let notes_scroller = build_notes_list(&list_box, sender);
    content_stack.add_named(&notes_scroller, Some("notes"));

    let empty_page = adw::StatusPage::new();
    empty_page.set_icon_name(Some("network-workgroup-symbolic"));
    content_stack.add_named(&empty_page, Some("empty"));

    let settings_page = build_settings_page(state, sender);
    content_stack.add_named(&settings_page.root, Some("settings"));

    let tags_page = build_tags_page();
    content_stack.add_named(&tags_page.root, Some("tags"));

    let timeline_page = build_timeline_page();
    content_stack.add_named(&timeline_page.root, Some("timeline"));

    // ── Focus page (shared reading/editing) ──

    let reading_page = build_reading_page(sender);
    content_stack.add_named(&reading_page.root, Some("reading"));

    // Wire up content change callback on the shared editor
    let mut editor = reading_page.editor;
    let sender_content = sender.input_sender().clone();
    editor.set_on_change(move |markdown| {
        let _ = sender_content.send(AppMsg::DraftContentChanged(markdown));
    });

    let editing_overlay = build_editing_overlay(sender);

    // ── Initial page ──

    let initial_child = initial_content_child(state);
    content_stack.set_visible_child_name(initial_child);

    ContentPages {
        content_stack,
        list_box,
        empty_page,

        reading_context_panel: reading_page.context_panel,
        reading_editor: editor,
        reading_tags_box: reading_page.tags_box,
        reading_meta_label: reading_page.meta_label,
        reading_origins_box: reading_page.origins_box,
        reading_replies_box: reading_page.replies_box,
        reading_versions_box: reading_page.versions_box,

        editing_title_label: editing_overlay.title_label,
        editing_hint_label: editing_overlay.hint_label,
        editing_tags_entry: editing_overlay.tags_entry,

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
    }
}

// ── Notes list (browse mode) ──

fn build_notes_list(list_box: &gtk::ListBox, sender: &ComponentSender<App>) -> gtk::ScrolledWindow {
    let scroller = gtk::ScrolledWindow::new();
    smooth_scroller(&scroller);
    scroller.set_child(Some(list_box));
    scroller.set_vexpand(true);

    let sender_scroll = sender.input_sender().clone();
    scroller.vadjustment().connect_value_changed(move |adj| {
        let upper = adj.upper();
        let page_size = adj.page_size();
        let value = adj.value();

        if upper > page_size && value >= upper - page_size - 100.0 {
            let _ = sender_scroll.send(AppMsg::LoadMoreNotes);
        }
    });

    scroller
}

// ── Reading page (focus mode) ──

struct ReadingPage {
    root: gtk::Paned,
    context_panel: gtk::ScrolledWindow,
    editor: WysiwygEditor,
    tags_box: gtk::Box,
    meta_label: gtk::Label,
    origins_box: gtk::Box,
    replies_box: gtk::Box,
    versions_box: gtk::Box,
}

fn build_reading_page(_sender: &ComponentSender<App>) -> ReadingPage {
    let root = gtk::Paned::new(gtk::Orientation::Horizontal);
    root.set_wide_handle(true);
    root.set_resize_start_child(true);
    root.set_shrink_start_child(false);
    root.set_shrink_end_child(false);

    // ── Left: article (WYSIWYG rendered markdown) ──

    let article_scroller = gtk::ScrolledWindow::new();
    smooth_scroller(&article_scroller);
    article_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    article_scroller.set_vexpand(true);

    // Overlay: content + floating tags at bottom
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&article_scroller));

    let clamp = adw::Clamp::builder()
        .maximum_size(960)
        .margin_top(32)
        .margin_bottom(64)
        .margin_start(32)
        .margin_end(32)
        .tightening_threshold(800)
        .build();

    let article = gtk::Box::new(gtk::Orientation::Vertical, 24);

    // Meta label — moved to header, kept here as hidden field for app.rs
    let meta_label = gtk::Label::new(None);
    meta_label.set_visible(false);

    let editor = WysiwygEditor::new_read_only();

    article.append(editor.widget());
    clamp.set_child(Some(&article));
    article_scroller.set_child(Some(&clamp));

    // Tags overlay — floating at bottom, semi-transparent
    let tags_overlay_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    tags_overlay_box.add_css_class("synap-reading-tags-overlay");
    tags_overlay_box.set_halign(gtk::Align::Center);
    tags_overlay_box.set_valign(gtk::Align::End);
    tags_overlay_box.set_margin_bottom(16);
    overlay.add_overlay(&tags_overlay_box);

    // ── Right: context panel ──

    let context_scroller = gtk::ScrolledWindow::new();
    smooth_scroller(&context_scroller);
    context_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    context_scroller.set_vexpand(true);
    context_scroller.set_min_content_width(280);
    context_scroller.set_max_content_width(380);

    let context_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
    context_box.set_margin_top(32);
    context_box.set_margin_bottom(32);
    context_box.set_margin_start(20);
    context_box.set_margin_end(20);

    let origins_box = build_detail_section("溯源链");
    let replies_box = build_detail_section("回复");
    let versions_box = build_detail_section("版本演化");

    context_box.append(&origins_box);
    context_box.append(&replies_box);
    context_box.append(&versions_box);
    context_scroller.set_child(Some(&context_box));

    // ── Assemble: article left, context right ──

    root.set_start_child(Some(&overlay));
    root.set_end_child(Some(&context_scroller));
    root.set_position(700);

    ReadingPage {
        root,
        context_panel: context_scroller,
        editor,
        tags_box: tags_overlay_box,
        meta_label,
        origins_box,
        replies_box,
        versions_box,
    }
}

// ── Editing overlay (hint + tags entry, shown in edit mode) ──

struct EditingOverlay {
    title_label: gtk::Label,
    hint_label: gtk::Label,
    tags_entry: gtk::Entry,
}

fn build_editing_overlay(sender: &ComponentSender<App>) -> EditingOverlay {
    let title_label = gtk::Label::new(None);

    let hint_label = gtk::Label::new(None);
    hint_label.add_css_class("caption");
    hint_label.add_css_class("dim-label");
    hint_label.set_halign(gtk::Align::Start);
    hint_label.set_xalign(0.0);
    hint_label.set_wrap(true);

    let tags_entry = gtk::Entry::new();
    tags_entry.set_placeholder_text(Some("标签，例如 rust, idea, diary"));
    let sender_tags = sender.input_sender().clone();
    tags_entry.connect_changed(move |entry| {
        let _ = sender_tags.send(AppMsg::DraftTagsChanged(entry.text().to_string()));
    });

    EditingOverlay {
        title_label,
        hint_label,
        tags_entry,
    }
}

// ── Settings page ──

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
    smooth_scroller(&root);
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

// ── Tags page ──

struct TagsPage {
    root: gtk::ScrolledWindow,
    tags_flow_box: gtk::FlowBox,
}

fn build_tags_page() -> TagsPage {
    let root = gtk::ScrolledWindow::new();
    smooth_scroller(&root);
    let tags_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
    tags_box.set_margin_top(32);
    tags_box.set_margin_bottom(32);
    tags_box.set_margin_start(28);
    tags_box.set_margin_end(28);
    tags_box.add_css_class("synap-tags-page");

    let tags_label = gtk::Label::new(Some("所有标签"));
    tags_label.add_css_class("heading");
    tags_label.add_css_class("synap-section-heading");
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

// ── Timeline page ──

struct TimelinePage {
    root: gtk::ScrolledWindow,
    timeline_container: gtk::Box,
}

fn build_timeline_page() -> TimelinePage {
    let root = gtk::ScrolledWindow::new();
    smooth_scroller(&root);
    let timeline_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
    timeline_box.set_margin_top(32);
    timeline_box.set_margin_bottom(32);
    timeline_box.set_margin_start(28);
    timeline_box.set_margin_end(28);
    timeline_box.add_css_class("synap-timeline-page");

    let timeline_container = gtk::Box::new(gtk::Orientation::Vertical, 24);
    timeline_box.append(&timeline_container);

    root.set_child(Some(&timeline_box));

    TimelinePage {
        root,
        timeline_container,
    }
}

// ── Helpers ──

fn build_detail_section(title: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
    section.add_css_class("synap-detail-section");
    let label = gtk::Label::new(Some(title));
    label.add_css_class("heading");
    label.add_css_class("synap-section-heading");
    label.set_halign(gtk::Align::Start);
    section.append(&label);
    section
}

fn initial_content_child(state: &AppState) -> &'static str {
    match state.content_view {
        ContentView::Notes => {
            if state.visible_notes().is_empty() {
                "empty"
            } else {
                "notes"
            }
        }
        ContentView::Trash => "notes",
        ContentView::Tags => "tags",
        ContentView::TagNotes => "notes",
        ContentView::Timeline => "timeline",
        ContentView::Settings => "settings",
    }
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("heading");
    label.add_css_class("synap-section-heading");
    label.set_halign(gtk::Align::Start);
    label.set_margin_top(12);
    label.set_margin_start(12);
    label
}
