//! Main application entry point for the desktop client.

use chrono::{Local, TimeZone};
use iced::{
    widget::{Button, Column, Container, Row, Scrollable, Text, TextInput},
    Element, Length, Settings, Task,
};
use synap_core::dto::NoteDTO;

use crate::{core::ServiceWrapper, message::Message, state::AppState};

pub fn main() -> iced::Result {
    if let Err(error) = ServiceWrapper::init() {
        eprintln!("Failed to initialize core service: {error}");
    }

    iced::application(
        || -> (AppState, Task<Message>) {
            let mut state = AppState::new();
            if let Err(error) = state.refresh() {
                state.set_status(format!("初始化失败: {error}"));
            }
            (state, Task::none())
        },
        update,
        view,
    )
    .settings(Settings::default())
    .run()
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Refresh => {
            if let Err(error) = state.refresh() {
                state.set_status(format!("刷新失败: {error}"));
            } else {
                state.set_status("已刷新");
            }
        }
        Message::SearchQueryChanged(value) => {
            state.search_query = value;
        }
        Message::RunSearch => {
            if let Err(error) = state.refresh() {
                state.set_status(format!("检索失败: {error}"));
            } else if state.search_query.trim().is_empty() {
                state.set_status("已回到最近笔记视图");
            } else {
                state.set_status(format!("已按“{}”检索", state.search_query.trim()));
            }
        }
        Message::ClearSearch => {
            state.search_query.clear();
            if let Err(error) = state.refresh() {
                state.set_status(format!("清空检索失败: {error}"));
            } else {
                state.set_status("已清空检索条件");
            }
        }
        Message::ComposeContentChanged(value) => {
            state.compose_content = value;
        }
        Message::ComposeTagsChanged(value) => {
            state.compose_tags = value;
        }
        Message::CreateNote => {
            let content = state.compose_content.trim().to_string();
            if content.is_empty() {
                state.set_status("请输入笔记内容");
                return Task::none();
            }

            match ServiceWrapper::create_note(content, parse_tags(&state.compose_tags)) {
                Ok(note) => {
                    state.compose_content.clear();
                    state.compose_tags.clear();
                    refresh_and_select(state, Some(note.id), "已创建笔记");
                }
                Err(error) => state.set_status(format!("创建失败: {error}")),
            }
        }
        Message::ReplyToSelected => {
            let Some(parent_id) = state.selected_note_id().map(str::to_owned) else {
                state.set_status("先选择一条笔记再回复");
                return Task::none();
            };
            let content = state.compose_content.trim().to_string();
            if content.is_empty() {
                state.set_status("请输入回复内容");
                return Task::none();
            }

            match ServiceWrapper::reply_note(&parent_id, content, parse_tags(&state.compose_tags)) {
                Ok(note) => {
                    state.compose_content.clear();
                    state.compose_tags.clear();
                    refresh_and_select(state, Some(note.id), "已创建回复");
                }
                Err(error) => state.set_status(format!("回复失败: {error}")),
            }
        }
        Message::SelectNote(id) => {
            if let Err(error) = state.select_note(&id) {
                state.set_status(format!("加载笔记失败: {error}"));
            }
        }
        Message::DetailContentChanged(value) => {
            state.detail_content = value;
        }
        Message::DetailTagsChanged(value) => {
            state.detail_tags = value;
        }
        Message::SaveNewVersion => {
            let Some(note_id) = state.selected_note_id().map(str::to_owned) else {
                state.set_status("没有可编辑的笔记");
                return Task::none();
            };
            let content = state.detail_content.trim().to_string();
            if content.is_empty() {
                state.set_status("编辑后的内容不能为空");
                return Task::none();
            }

            match ServiceWrapper::edit_note(&note_id, content, parse_tags(&state.detail_tags)) {
                Ok(note) => refresh_and_select(state, Some(note.id), "已生成新版本"),
                Err(error) => state.set_status(format!("生成新版本失败: {error}")),
            }
        }
        Message::DeleteSelected => {
            let Some(note_id) = state.selected_note_id().map(str::to_owned) else {
                state.set_status("先选择一条笔记再删除");
                return Task::none();
            };

            match ServiceWrapper::delete_note(&note_id) {
                Ok(()) => {
                    state.clear_selection();
                    refresh_and_select(state, None, "已标记删除，笔记会出现在删除列表");
                }
                Err(error) => state.set_status(format!("删除失败: {error}")),
            }
        }
        Message::RestoreNote(id) => match ServiceWrapper::restore_note(&id) {
            Ok(()) => refresh_and_select(state, Some(id), "已恢复笔记"),
            Err(error) => state.set_status(format!("恢复失败: {error}")),
        },
        Message::SeedDemo => {
            if let Err(error) = seed_demo(state) {
                state.set_status(format!("写入示例失败: {error}"));
            } else {
                state.set_status("已写入一组演示数据");
            }
        }
    }

    Task::none()
}

fn view(state: &AppState) -> Element<'_, Message> {
    let header = Row::new()
        .spacing(12)
        .push(Text::new("Synap Desktop").size(28))
        .push(Text::new("current core service demo").size(16))
        .push(Container::new(Text::new("")).width(Length::Fill))
        .push(Button::new(Text::new("刷新")).on_press(Message::Refresh))
        .push(Button::new(Text::new("写入示例")).on_press(Message::SeedDemo));

    let status = state
        .status
        .as_ref()
        .map(|message| Container::new(Text::new(message.as_str())).padding(8));

    let capture_panel = Container::new(
        Column::new()
            .spacing(10)
            .push(Text::new("快速创建 / 回复").size(22))
            .push(Text::new("内容").size(14))
            .push(
                TextInput::new("输入 Markdown 内容", &state.compose_content)
                    .on_input(Message::ComposeContentChanged)
                    .padding(10),
            )
            .push(Text::new("标签").size(14))
            .push(
                TextInput::new("用逗号分隔，如 demo, rust", &state.compose_tags)
                    .on_input(Message::ComposeTagsChanged)
                    .padding(10),
            )
            .push(
                Row::new()
                    .spacing(8)
                    .push(Button::new(Text::new("新建笔记")).on_press(Message::CreateNote))
                    .push(reply_button(state)),
            )
            .push(Text::new("已删除").size(20))
            .push(Text::new("删除仍由 service 过滤，单独列表用于恢复。").size(12))
            .push(deleted_notes_view(&state.deleted_notes)),
    )
    .padding(16)
    .width(Length::FillPortion(2));

    let notes_panel = Container::new(
        Column::new()
            .spacing(10)
            .push(Text::new("最近笔记 / 搜索").size(22))
            .push(
                Row::new()
                    .spacing(8)
                    .push(
                        TextInput::new("搜索内容或标签", &state.search_query)
                            .on_input(Message::SearchQueryChanged)
                            .on_submit(Message::RunSearch)
                            .padding(10)
                            .width(Length::Fill),
                    )
                    .push(Button::new(Text::new("搜索")).on_press(Message::RunSearch))
                    .push(Button::new(Text::new("清空")).on_press(Message::ClearSearch)),
            )
            .push(Text::new(format!(
                "当前显示 {} 条可见笔记",
                state.notes.len()
            )))
            .push(notes_list_view(state)),
    )
    .padding(16)
    .width(Length::FillPortion(3));

    let detail_panel = Container::new(detail_view(state))
        .padding(16)
        .width(Length::FillPortion(3));

    let mut content = Column::new().spacing(16).padding(16).push(header);
    if let Some(status) = status {
        content = content.push(status);
    }
    content = content.push(
        Row::new()
            .spacing(16)
            .height(Length::Fill)
            .push(capture_panel)
            .push(notes_panel)
            .push(detail_panel),
    );

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn notes_list_view(state: &AppState) -> Element<'_, Message> {
    let mut list = Column::new().spacing(8);

    if state.notes.is_empty() {
        list = list.push(Text::new(
            "还没有可展示的笔记。可以先写入示例，或者直接创建。",
        ));
    } else {
        for note in &state.notes {
            let is_selected = state
                .selected_note
                .as_ref()
                .is_some_and(|selected| selected.id == note.id);
            list = list.push(note_card(
                note,
                is_selected,
                Message::SelectNote(note.id.clone()),
            ));
        }
    }

    Scrollable::new(list).height(Length::Fill).into()
}

fn detail_view(state: &AppState) -> Element<'_, Message> {
    let Some(note) = state.selected_note.as_ref() else {
        return Container::new(
            Column::new()
                .spacing(12)
                .push(Text::new("详情").size(22))
                .push(Text::new("选中一条笔记后，这里会展示："))
                .push(Text::new("1. 当前内容与标签"))
                .push(Text::new("2. 基于编辑链的其他版本"))
                .push(Text::new("3. 父链溯源与回复列表")),
        )
        .into();
    };

    let mut panel = Column::new()
        .spacing(12)
        .push(Text::new("选中笔记").size(22))
        .push(Text::new(format!("ID: {}", note.id)).size(12))
        .push(Text::new(format!("创建时间: {}", format_timestamp(note.created_at))).size(12))
        .push(Text::new(format!("标签: {}", format_tags(&note.tags))))
        .push(Text::new("编辑会生成新版本，而不是原地覆盖。").size(12))
        .push(
            TextInput::new("编辑内容", &state.detail_content)
                .on_input(Message::DetailContentChanged)
                .padding(10),
        )
        .push(
            TextInput::new("编辑后的标签", &state.detail_tags)
                .on_input(Message::DetailTagsChanged)
                .padding(10),
        )
        .push(
            Row::new()
                .spacing(8)
                .push(Button::new(Text::new("生成新版本")).on_press(Message::SaveNewVersion))
                .push(Button::new(Text::new("标记删除")).on_press(Message::DeleteSelected)),
        )
        .push(relationship_section("父链溯源", &state.selected_origins))
        .push(relationship_section("回复", &state.selected_replies))
        .push(relationship_section(
            "其他版本",
            &state
                .selected_versions
                .iter()
                .map(|version| version.note.clone())
                .collect::<Vec<_>>(),
        ));

    panel = panel.push(Text::new("当前内容预览").size(18));
    panel = panel.push(Container::new(Text::new(note.content.as_str())).padding(12));

    Scrollable::new(panel).height(Length::Fill).into()
}

fn deleted_notes_view(notes: &[NoteDTO]) -> Element<'_, Message> {
    let mut list = Column::new().spacing(8);

    if notes.is_empty() {
        list = list.push(Text::new("删除列表为空"));
    } else {
        for note in notes {
            list = list.push(
                Row::new()
                    .spacing(8)
                    .push(Container::new(Text::new(preview(&note.content, 36))).width(Length::Fill))
                    .push(
                        Button::new(Text::new("恢复"))
                            .on_press(Message::RestoreNote(note.id.clone())),
                    ),
            );
        }
    }

    Scrollable::new(list).height(180).into()
}

fn relationship_section<'a>(title: &'a str, notes: &'a [NoteDTO]) -> Element<'a, Message> {
    let mut section = Column::new().spacing(8).push(Text::new(title).size(18));

    if notes.is_empty() {
        section = section.push(Text::new("暂无"));
    } else {
        for note in notes {
            section = section.push(note_card(note, false, Message::SelectNote(note.id.clone())));
        }
    }

    Container::new(section).into()
}

fn note_card<'a>(note: &'a NoteDTO, selected: bool, message: Message) -> Element<'a, Message> {
    let prefix = if selected { "当前选中" } else { "笔记" };
    let content = Column::new()
        .spacing(4)
        .push(Text::new(prefix).size(12))
        .push(Text::new(preview(&note.content, 64)).size(16))
        .push(Text::new(format_tags(&note.tags)).size(12))
        .push(Text::new(format_timestamp(note.created_at)).size(12));

    Button::new(Container::new(content).padding(12).width(Length::Fill))
        .width(Length::Fill)
        .on_press(message)
        .into()
}

fn reply_button(state: &AppState) -> Element<'_, Message> {
    if state.selected_note.is_some() {
        Button::new(Text::new("回复当前选中"))
            .on_press(Message::ReplyToSelected)
            .into()
    } else {
        Button::new(Text::new("先选中一条笔记")).into()
    }
}

fn refresh_and_select(state: &mut AppState, select_id: Option<String>, success_message: &str) {
    match state.refresh() {
        Ok(()) => {
            if let Some(id) = select_id {
                if let Err(error) = state.select_note(&id) {
                    state.set_status(format!("{success_message}，但重新加载失败: {error}"));
                    return;
                }
            }
            state.set_status(success_message);
        }
        Err(error) => state.set_status(format!("{success_message}，但刷新失败: {error}")),
    }
}

fn seed_demo(state: &mut AppState) -> crate::core::Result<()> {
    let root = ServiceWrapper::create_note(
        "桌面 demo：这是第一版根笔记。".to_string(),
        vec!["demo".to_string(), "desktop".to_string()],
    )?;
    let edited = ServiceWrapper::edit_note(
        &root.id,
        "桌面 demo：这是第二版根笔记，用来展示“编辑即新版本”。".to_string(),
        vec!["demo".to_string(), "version".to_string()],
    )?;
    let _reply = ServiceWrapper::reply_note(
        &edited.id,
        "这是根笔记的一个回复，用来演示 service.get_replies。".to_string(),
        vec!["reply".to_string(), "demo".to_string()],
    )?;
    let deleted = ServiceWrapper::create_note(
        "这条笔记会被删除，然后从右侧的删除列表中恢复。".to_string(),
        vec!["deleted".to_string(), "demo".to_string()],
    )?;
    ServiceWrapper::delete_note(&deleted.id)?;

    state.refresh()?;
    state.select_note(&edited.id)?;
    Ok(())
}

fn parse_tags(input: &str) -> Vec<String> {
    input
        .split([',', '，', '\n'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

fn preview(content: &str, limit: usize) -> String {
    let flattened = content.replace('\n', " ");
    let mut chars = flattened.chars();
    let preview: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        "无标签".to_string()
    } else {
        tags.iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let ts = timestamp as i64;
    let maybe_time = if ts > 10_000_000_000 {
        Local.timestamp_millis_opt(ts).single()
    } else {
        Local.timestamp_opt(ts, 0).single()
    };

    maybe_time
        .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}
