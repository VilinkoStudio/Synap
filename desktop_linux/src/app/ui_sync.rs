//! UI sync — synchronize widget state from AppState.

use adw::prelude::*;
use relm4::prelude::*;

use super::App;
use super::message::AppMsg;
use crate::{
    domain::ContentView,
    ui::{
        note_widgets::{build_clickable_note_row, build_note_card, build_timeline_header, tag_chip},
        util::render_reading_text,
    },
};

impl App {
    pub(super) fn sync_ui(&self, sender: &ComponentSender<Self>) {
        self.sync_content_stack();
        self.sync_empty_page();
        self.sync_home_feed(sender);
        self.sync_reading(sender);
        self.sync_editing();
        self.sync_theme_dropdown();
        self.sync_settings(sender);
    }

    fn sync_content_stack(&self) {
        if !self.state.focus_mode.is_browse() {
            return;
        }

        let is_empty = self.state.visible_notes().is_empty();
        let child_name = match self.state.content_view {
            ContentView::Notes | ContentView::Trash => {
                if is_empty { "empty" } else { "notes" }
            }
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
        self.list.empty_page.set_title(title);
        self.list.empty_page.set_description(Some(&desc));
    }

    fn sync_reading(&self, sender: &ComponentSender<Self>) {
        self.reading.context_panel
            .set_visible(self.state.context_panel_open);
        // Only sync editor content in reading mode.
        // In editing mode, sync_editing handles the editor content.
        // Writing rendered text here would conflict with raw markdown in sync_editing,
        // causing an infinite connect_changed → sync_ui loop.
        if !self.state.focus_mode.is_editing() {
            self.reading.editor.borrow_mut().set_content(&self.reading_content());
        }
        self.reading.meta_label.set_text(&self.reading_meta());

        clear_box(&self.reading.tags_box);
        if let Some(detail) = &self.state.selected_note_detail {
            if detail.tags.is_empty() {
                let label = gtk::Label::new(Some("暂无标签"));
                label.add_css_class("caption");
                label.add_css_class("dim-label");
                self.reading.tags_box.append(&label);
            } else {
                for tag in &detail.tags {
                    self.reading.tags_box.append(&tag_chip(tag));
                }
            }
        }

        self.sync_note_section(&self.reading.origins_box, "无溯源", |full| &full.origins, sender);
        self.sync_note_section(&self.reading.replies_box, "无回复", |full| &full.replies, sender);

        while self.reading.versions_box.observe_children().n_items() > 1 {
            if let Some(child) = self.reading.versions_box.last_child() {
                self.reading.versions_box.remove(&child);
            }
        }
        if let Some(full) = &self.state.selected_note_full {
            if full.other_versions.is_empty() {
                self.reading.versions_box.append(&empty_relation_label("无其他版本"));
            } else {
                for version in &full.other_versions {
                    let note = &version.note;
                    self.reading.versions_box.append(&build_clickable_note_row(
                        note, sender.input_sender(), note.id.clone(),
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
                    container.append(&build_clickable_note_row(note, sender.input_sender(), note.id.clone()));
                }
            }
        }
    }

    fn sync_editing(&self) {
        if self.state.focus_mode.is_editing() {
            let editor = self.reading.editor.borrow();
            if editor.content() != self.state.draft_content {
                editor.set_content(&self.state.draft_content);
            }
        }
        if self.editing.tags_entry.text().as_str() != self.state.draft_tags_text {
            self.editing.tags_entry.set_text(&self.state.draft_tags_text);
        }

        // Sync recommended tags (only visible in editing mode)
        clear_box(&self.editing.recommend_tags_box);
        if self.state.focus_mode.is_editing() && !self.state.recommended_tags.is_empty() {
            let existing_tags: Vec<String> = self.state.draft_tags_text
                .split([',', '，'])
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            for tag in &self.state.recommended_tags {
                if existing_tags.contains(&tag.to_lowercase()) {
                    continue;
                }
                let btn = gtk::Button::with_label(&format!("+{tag}"));
                btn.add_css_class("pill");
                btn.add_css_class("flat");
                btn.set_tooltip_text(Some("点击添加此标签"));
                let tag_clone = tag.clone();
                let current = self.editing.tags_entry.text().to_string();
                // We can't directly modify state here, but the button click
                // will trigger DraftTagsChanged via the entry's connect_changed
                let entry = self.editing.tags_entry.clone();
                btn.connect_clicked(move |_| {
                    let mut tags = entry.text().to_string();
                    if !tags.is_empty() && !tags.ends_with(',') && !tags.ends_with('，') {
                        tags.push_str(", ");
                    }
                    tags.push_str(&tag_clone);
                    entry.set_text(&tags);
                });
                self.editing.recommend_tags_box.append(&btn);
            }
        }
    }

    fn sync_theme_dropdown(&self) {
        let idx = self.state.theme.index();
        if self.settings.theme_dropdown.selected() != idx {
            self.settings.theme_dropdown.set_selected(idx);
        }
    }

    fn sync_settings(&self, sender: &ComponentSender<Self>) {
        let listener = &self.state.sync.listener;
        self.settings.listener_row.set_subtitle(&format!(
            "{}{}",
            listener.status,
            listener.listen_port.map(|port| format!(" · 端口 {port}")).unwrap_or_default()
        ));
        let addresses = if listener.local_addresses.is_empty() {
            "未获取到局域网地址".to_string()
        } else {
            listener.local_addresses.join(", ")
        };
        self.settings.addresses_row.set_subtitle(&addresses);

        self.settings.identity_row.set_subtitle(
            self.state.sync.local_identity.as_ref()
                .map(|id| id.identity.kaomoji_fingerprint.as_str())
                .unwrap_or("—"),
        );
        self.settings.signing_row.set_subtitle(
            self.state.sync.local_identity.as_ref()
                .map(|id| id.signing.kaomoji_fingerprint.as_str())
                .unwrap_or("—"),
        );

        self.settings.error_label.set_visible(self.state.sync.error_message.is_some());
        self.settings.error_label.set_text(self.state.sync.error_message.as_deref().unwrap_or(""));

        // Relay config sync (avoid overwriting user input while typing)
        if self.settings.relay_base_url_entry.text().as_str() != self.state.sync.relay_base_url {
            self.settings.relay_base_url_entry.set_text(&self.state.sync.relay_base_url);
        }
        if self.settings.relay_api_key_entry.text().as_str() != self.state.sync.relay_api_key {
            self.settings.relay_api_key_entry.set_text(&self.state.sync.relay_api_key);
        }
        self.settings
            .relay_status_label
            .set_visible(self.state.sync.relay_status_message.is_some());
        self.settings.relay_status_label.set_text(
            self.state.sync.relay_status_message.as_deref().unwrap_or(""),
        );

        if self.settings.host_entry.text().as_str() != self.state.sync.host_input {
            self.settings.host_entry.set_text(&self.state.sync.host_input);
        }
        if self.settings.port_entry.text().as_str() != self.state.sync.port_input {
            self.settings.port_entry.set_text(&self.state.sync.port_input);
        }

        self.sync_settings_discovered(sender);
        self.sync_settings_connections(sender);
        self.sync_settings_peers(sender);
        self.sync_settings_sessions();
    }

    fn sync_settings_discovered(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.settings.discovered_box);
        if self.state.sync.discovered_peers.is_empty() {
            self.settings.discovered_box.append(&simple_info_row(
                "暂无发现设备", "确认设备在同一局域网并已启动监听",
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
                let _ = s.send(AppMsg::PairDiscoveredPeer { host: host.clone(), port });
            });
            row.add_suffix(&button);
            self.settings.discovered_box.append(&row);
        }
    }

    fn sync_settings_connections(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.settings.connections_box);
        if self.state.sync.connections.is_empty() {
            self.settings.connections_box.append(&simple_info_row(
                "暂无已保存连接", "可手动输入主机地址与端口添加",
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
            self.settings.connections_box.append(&row);
        }
    }

    fn sync_settings_peers(&self, sender: &ComponentSender<Self>) {
        clear_box(&self.settings.peers_box);
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
                let _ = s.send(AppMsg::TrustPeer { public_key: pk.clone(), note: None });
            });
            row.add_suffix(&button);
            self.settings.peers_box.append(&row);
        }
        if self.state.sync.peers.is_empty() {
            self.settings.peers_box.append(&simple_info_row(
                "还没有设备记录", "首次配对后会在这里显示公钥与信任状态",
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
                .title("删除设备记录").subtitle("移除本地记录").activatable(true).build();
            let ds = sender.input_sender().clone();
            let did = peer.id.clone();
            let g = gtk::GestureClick::new();
            g.connect_released(move |_, _, _, _| {
                let _ = ds.send(AppMsg::DeletePeer(did.clone()));
            });
            del_row.add_controller(g);
            row.add_row(&del_row);

            self.settings.peers_box.append(&row);
        }
    }

    fn sync_settings_sessions(&self) {
        clear_box(&self.settings.sessions_box);
        if self.state.sync.recent_sessions.is_empty() {
            self.settings.sessions_box.append(&simple_info_row(
                "暂无同步记录", "发起或接收一次同步后会显示在这里",
            ));
            return;
        }
        for session in &self.state.sync.recent_sessions {
            let transport_info = crate::domain::transport_label(&session.transport);
            let relay_info = session
                .relay_url
                .as_deref()
                .filter(|u| !u.is_empty())
                .map(|u| format!(" · {u}"))
                .unwrap_or_default();
            self.settings.sessions_box.append(
                &adw::ActionRow::builder()
                    .title(session.peer_label.as_deref().unwrap_or("未知设备"))
                    .subtitle(format!(
                        "{} · {} · {} · {transport_info}{relay_info}",
                        crate::domain::sync_role_label(&session.role),
                        crate::domain::sync_status_label(&session.status),
                        crate::domain::format_timestamp(session.finished_at_ms),
                    ))
                    .build(),
            );
        }
    }

    fn sync_home_feed(&self, sender: &ComponentSender<Self>) {
        // Sync home feed for Notes and Trash views in browse mode
        if !matches!(self.state.content_view, ContentView::Notes | ContentView::Trash)
            || !self.state.focus_mode.is_browse()
        {
            return;
        }

        let is_notes = self.state.content_view == ContentView::Notes;

        // ── Tag filter chips (only for Notes view) ──
        clear_box(&self.home_tag_box);
        self.home_tag_box.set_visible(is_notes);

        if is_notes {
            let filter = &self.state.home.tag_filter;
            let is_all = !filter.tag_filter_enabled;
            let all_btn = gtk::Button::with_label("全部");
            all_btn.add_css_class("pill");
            if is_all {
                all_btn.add_css_class("accent");
            }
            let s_all = sender.input_sender().clone();
            all_btn.connect_clicked(move |_| {
                let _ = s_all.send(AppMsg::ToggleAllTags);
            });
            self.home_tag_box.append(&all_btn);

            let untagged_btn = gtk::Button::with_label("无标签");
            untagged_btn.add_css_class("pill");
            if filter.tag_filter_enabled && !filter.include_untagged {
                untagged_btn.add_css_class("accent");
            }
            let s_untagged = sender.input_sender().clone();
            untagged_btn.connect_clicked(move |_| {
                let _ = s_untagged.send(AppMsg::ToggleUntaggedFilter);
            });
            self.home_tag_box.append(&untagged_btn);

            for tag in &self.state.home.all_tags {
                let btn = gtk::Button::with_label(&format!("#{tag}"));
                btn.add_css_class("pill");
                let is_selected = !filter.tag_filter_enabled || filter.selected_tags.contains(tag);
                if filter.tag_filter_enabled && is_selected {
                    btn.add_css_class("accent");
                }
                let tc = tag.clone();
                let s = sender.input_sender().clone();
                btn.connect_clicked(move |_| {
                    let _ = s.send(AppMsg::ToggleTagFilter(tc.clone()));
                });
                self.home_tag_box.append(&btn);
            }
        }

        // ── FlowBox: timeline headers + note cards ──
        while let Some(child) = self.home_flow_box.first_child() {
            self.home_flow_box.remove(&child);
        }

        let notes = self.state.visible_notes();
        let is_trash = self.state.content_view == ContentView::Trash;
        for note in &notes {
            // Insert timeline group header when a new group starts (Notes view only)
            if is_notes {
                if let Some(group) = &note.timeline_group {
                    if group.starts_group {
                        let header = build_timeline_header(
                            group.started_at as u64,
                            group.ended_at as u64,
                            group.note_count,
                        );
                        self.home_flow_box.insert(&header, -1);
                    }
                }
            }

            let card = build_note_card(note, sender.input_sender(), is_trash);
            self.home_flow_box.insert(&card, -1);
        }

        // Loading spinner at the end
        if self.state.is_loading_more {
            let spinner = gtk::Spinner::new();
            spinner.set_margin_top(8);
            spinner.set_margin_bottom(8);
            spinner.start();
            self.home_flow_box.insert(&spinner, -1);
        }
    }

    pub(super) fn reading_content(&self) -> String {
        self.state.selected_note_detail.as_ref()
            .map(|d| render_reading_text(&d.content))
            .unwrap_or_else(|| "请先从列表中打开一条笔记。".to_string())
    }

    pub(super) fn reading_meta(&self) -> String {
        self.state.selected_note_detail.as_ref()
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

// ── GTK helpers (shared by ui_sync and other modules) ──

pub(super) fn clear_box(container: &gtk::Box) {
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

pub(super) fn simple_info_row(title: &str, subtitle: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build()
}
