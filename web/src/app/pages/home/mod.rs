use leptos::{prelude::*, task::spawn_local};
use synap_core::NoteDTO;

use crate::app::server::{
    create_note_server, delete_note_server, edit_note_server, list_notes_page,
    recommend_tags_server,
};
use crate::app::ui::{Button, ButtonVariant, ConfirmDialog, SearchBox};
use crate::app::utils::{format_timestamp, short_note_id};

mod components;
mod state;

use components::{
    EditorActions, NoteList, NoteViewer, SettingsPanel, SidebarHeader, StatusBar, TagEditor,
    TagSuggestions,
};
use state::{EditSession, EditorDraft, HomeMode, PendingAction};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConfirmState {
    Discard(PendingAction),
    Delete(NoteDTO),
}

const DEFAULT_SIDEBAR_WIDTH: i32 = 320;
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
const MIN_SIDEBAR_WIDTH: i32 = 320;
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
const MAIN_CONTENT_MIN_WIDTH: i32 = 400;
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
const RECOMMENDATION_DEBOUNCE_MS: i32 = 400;

#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
fn clamp_sidebar_width(window_width: i32, width: i32) -> i32 {
    let max_width = (window_width - MAIN_CONTENT_MIN_WIDTH).max(MIN_SIDEBAR_WIDTH);
    width.clamp(MIN_SIDEBAR_WIDTH, max_width)
}

#[cfg(feature = "hydrate")]
fn current_window_width() -> Option<i32> {
    web_sys::window()?
        .inner_width()
        .ok()?
        .as_f64()
        .map(|value| value as i32)
}

#[component]
pub fn HomePage() -> impl IntoView {
    let (search_input, set_search_input) = signal(String::new());
    let (active_query, set_active_query) = signal(String::new());
    let (refresh_key, set_refresh_key) = signal(0_u64);
    let (home_mode, set_home_mode) = signal(HomeMode::Empty);
    let (status, set_status) = signal(String::from("Synap Web 已连接到 synap-core"));
    let (is_saving, set_is_saving) = signal(false);
    let (is_deleting, set_is_deleting) = signal(false);
    #[cfg_attr(not(feature = "hydrate"), allow(unused_variables))]
    let (sidebar_width, set_sidebar_width) = signal(DEFAULT_SIDEBAR_WIDTH);
    let (is_resizing, set_is_resizing) = signal(false);
    let (notes, set_notes) = signal(Vec::<NoteDTO>::new());
    let (notes_next_cursor, set_notes_next_cursor) = signal(None::<String>);
    let (notes_loading, set_notes_loading) = signal(true);
    let (notes_loading_more, set_notes_loading_more) = signal(false);
    let (notes_error, set_notes_error) = signal(None::<String>);
    let (confirm_state, set_confirm_state) = signal(None::<ConfirmState>);
    let (debounced_content, set_debounced_content) = signal(String::new());
    #[cfg(feature = "hydrate")]
    let recommendation_debounce = RwSignal::new(None::<i32>);

    let active_note = Memo::new(move |_| home_mode.get().active_note());
    let draft = Memo::new(move |_| home_mode.get().draft());
    let is_mutating = Signal::derive(move || home_mode.get().is_mutating());
    let is_dirty = Signal::derive(move || home_mode.get().is_dirty());

    let editor_title = Memo::new(move |_| match home_mode.get() {
        HomeMode::Viewing(_) => "查看笔记".to_string(),
        HomeMode::Editing(_) => "编辑笔记".to_string(),
        HomeMode::Creating(_) => "新建笔记".to_string(),
        HomeMode::Settings => "设置".to_string(),
        HomeMode::Empty => "笔记".to_string(),
    });

    let editor_subtitle = Memo::new(move |_| match home_mode.get() {
        HomeMode::Viewing(note) | HomeMode::Editing(EditSession { original: note, .. }) => format!(
            "{} · {}",
            short_note_id(&note.id),
            format_timestamp(note.created_at)
        ),
        HomeMode::Creating(_) => "写完后手动保存，才会生成新的笔记版本。".to_string(),
        HomeMode::Settings => "调整数据与界面设置".to_string(),
        HomeMode::Empty => "还没有可显示的笔记".to_string(),
    });

    let save_label = Memo::new(move |_| match home_mode.get() {
        HomeMode::Editing(_) => "保存为新版本".to_string(),
        HomeMode::Creating(_) => "创建笔记".to_string(),
        HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => "保存".to_string(),
    });

    let can_save = Signal::derive(move || {
        is_mutating.get()
            && !is_saving.get()
            && !is_deleting.get()
            && draft
                .get()
                .is_some_and(|current| !current.content.trim().is_empty())
    });

    let clear_recommendation_timer = Callback::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(handle) = recommendation_debounce.get_untracked() {
                if let Some(window) = web_sys::window() {
                    window.clear_timeout_with_handle(handle);
                }
                recommendation_debounce.set(None);
            }
        }
    });

    let navigate = {
        let set_home_mode = set_home_mode;
        let set_status = set_status;
        Callback::new(move |target: PendingAction| match target {
            PendingAction::View(note) => {
                set_home_mode.set(HomeMode::Viewing(note.clone()));
                set_status.set(format!("已选择 {}", short_note_id(&note.id)));
            }
            PendingAction::Edit(note) => {
                set_home_mode.set(HomeMode::Editing(EditSession::from_note(note.clone())));
                set_status.set(format!("正在编辑 {}", short_note_id(&note.id)));
            }
            PendingAction::Create => {
                set_home_mode.set(HomeMode::Creating(EditorDraft::empty()));
                set_status.set("正在创建新笔记".to_string());
            }
            PendingAction::Settings => {
                set_home_mode.set(HomeMode::Settings);
                set_status.set("已打开设置".to_string());
            }
        })
    };

    let request_navigation = {
        let navigate = navigate.clone();
        let set_confirm_state = set_confirm_state;
        Callback::new(move |target: PendingAction| {
            let current_mode = home_mode.get_untracked();

            let is_same_target = match (&current_mode, &target) {
                (HomeMode::Settings, PendingAction::Settings) => true,
                (HomeMode::Creating(_), PendingAction::Create) => true,
                (HomeMode::Viewing(current), PendingAction::View(next)) => current.id == next.id,
                (
                    HomeMode::Editing(EditSession {
                        original: current, ..
                    }),
                    PendingAction::Edit(next),
                ) => current.id == next.id,
                // 其他组合一律视为 false
                _ => false,
            };

            if is_same_target {
                return;
            }

            if current_mode.is_dirty() {
                set_confirm_state.set(Some(ConfirmState::Discard(target)));
            } else {
                navigate.run(target);
            }
        })
    };

    Effect::new(move |_| {
        let query = active_query.get();
        let refresh = refresh_key.get();

        set_notes_loading.set(true);
        set_notes_loading_more.set(false);
        set_notes_error.set(None);
        set_notes.set(Vec::new());
        set_notes_next_cursor.set(None);

        spawn_local(async move {
            let result = list_notes_page(query.clone(), None).await;
            if active_query.get_untracked() != query || refresh_key.get_untracked() != refresh {
                return;
            }

            set_notes_loading.set(false);

            match result {
                Ok(page) => {
                    set_notes.set(page.notes);
                    set_notes_next_cursor.set(page.next_cursor);
                }
                Err(error) => {
                    set_notes_error.set(Some(error.to_string()));
                }
            }
        });
    });

    Effect::new(move |_| {
        if notes_loading.get() || notes_error.get().is_some() {
            return;
        }

        if matches!(home_mode.get(), HomeMode::Empty) {
            if let Some(first) = notes.get().into_iter().next() {
                set_home_mode.set(HomeMode::Viewing(first));
            }
        }
    });

    Effect::new(move |_| {
        let next_content = match home_mode.get() {
            HomeMode::Editing(session) => session.draft.content.trim().to_string(),
            HomeMode::Creating(draft) => draft.content.trim().to_string(),
            HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => String::new(),
        };

        clear_recommendation_timer.run(());

        if next_content.is_empty() {
            set_debounced_content.set(String::new());
            return;
        }

        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;

            let Some(window) = web_sys::window() else {
                set_debounced_content.set(next_content);
                return;
            };

            let delayed_content = next_content.clone();
            let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
                set_debounced_content.set(delayed_content);
            });

            match window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                RECOMMENDATION_DEBOUNCE_MS,
            ) {
                Ok(handle) => recommendation_debounce.set(Some(handle)),
                Err(_) => set_debounced_content.set(next_content),
            }
        }

        #[cfg(not(feature = "hydrate"))]
        {
            set_debounced_content.set(next_content);
        }
    });

    let recommended_tags = Resource::new(
        move || debounced_content.get(),
        |content| async move {
            if content.is_empty() {
                Ok(Vec::new())
            } else {
                recommend_tags_server(content).await
            }
        },
    );

    let available_recommended_tags = Memo::new(move |_| {
        let existing = draft.get().map(|current| current.tags).unwrap_or_default();
        match recommended_tags.get() {
            Some(Ok(tags)) => tags
                .into_iter()
                .filter(|tag| !existing.iter().any(|current| current == tag))
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        }
    });

    view! {
        <Transition
            fallback=move || view! { <div class="app-fallback">"正在从 synap-core 载入数据..."</div> }
        >
            <div
                class="synap-app"
                on:mousemove=move |_ev| {
                    if !is_resizing.get() {
                        return;
                    }

                    #[cfg(feature = "hydrate")]
                    {
                        let width = current_window_width().unwrap_or(DEFAULT_SIDEBAR_WIDTH);
                        let next = clamp_sidebar_width(width, _ev.client_x());
                        set_sidebar_width.set(next);
                    }
                }
                on:mouseup=move |_| {
                    if !is_resizing.get() {
                        return;
                    }

                    set_is_resizing.set(false);

                    #[cfg(feature = "hydrate")]
                    {
                        if let Some(window) = web_sys::window() {
                            if let Ok(Some(storage)) = window.local_storage() {
                                let _ = storage.set_item(
                                    "synap_sidebar_width",
                                    &sidebar_width.get_untracked().to_string(),
                                );
                            }
                        }
                    }
                }
                on:mouseleave=move |_| {
                    if is_resizing.get() {
                        set_is_resizing.set(false);
                    }
                }
            >
                <div class="sidebar" id="sidebar" style:width=move || format!("{}px", sidebar_width.get())>
                    <SidebarHeader
                        title="Synap Web"
                        on_open_settings=Callback::new(move |_| request_navigation.run(PendingAction::Settings))
                        on_refresh=Callback::new(move |_| {
                            set_refresh_key.update(|value| *value += 1);
                            set_status.set("已刷新笔记列表".to_string());
                        })
                        on_create=Callback::new(move |_| request_navigation.run(PendingAction::Create))
                    />

                    <SearchBox
                        search_input
                        set_search_input
                        on_search=Callback::new(move |_| {
                            let normalized = search_input.get_untracked().trim().to_string();
                            if active_query.get_untracked() == normalized {
                                set_refresh_key.update(|value| *value += 1);
                            } else {
                                set_active_query.set(normalized.clone());
                            }

                            if normalized.is_empty() {
                                set_status.set("已切回最近笔记流".to_string());
                            } else {
                                set_status.set(format!("已按“{normalized}”检索"));
                            }
                        })
                    />

                    <div class="list-title">
                        {move || {
                            let items = notes.get();
                            if notes_loading.get() && items.is_empty() {
                                "正在加载笔记…".to_string()
                            } else if active_query.get().is_empty() {
                                format!("最近笔记 · {} 条", items.len())
                            } else {
                                format!("搜索结果 · {} 条", items.len())
                            }
                        }}
                    </div>

                    <div class="sidebar-scroll-area">
                        {move || {
                            let items = notes.get();

                            if notes_loading.get() && items.is_empty() {
                                view! { <div class="list-empty">"正在从 synap-core 载入数据..."</div> }.into_any()
                            } else if let Some(error) = notes_error.get() {
                                view! { <div class="list-empty error-text">{format!("加载失败: {error}")}</div> }
                                    .into_any()
                            } else if items.is_empty() {
                                let empty_text = if active_query.get().is_empty() {
                                    "还没有笔记，点击右上角 + 开始记录。"
                                } else {
                                    "没有匹配结果，试试别的关键词。"
                                };

                                view! { <div class="list-empty">{empty_text}</div> }.into_any()
                            } else {
                                view! {
                                    <div class="note-list-container">
                                        <NoteList
                                            notes=Signal::derive(move || notes.get())
                                            active_note
                                            actions_disabled=Signal::derive(move || is_deleting.get())
                                            on_select=Callback::new(move |note: NoteDTO| {
                                                request_navigation.run(PendingAction::View(note));
                                            })
                                            on_edit=Callback::new(move |note: NoteDTO| {
                                                request_navigation.run(PendingAction::Edit(note));
                                            })
                                            on_delete=Callback::new(move |note: NoteDTO| {
                                                set_confirm_state.set(Some(ConfirmState::Delete(note)));
                                            })
                                        />

                                        <Show
                                            when=move || {
                                                active_query.get().is_empty()
                                                    && notes_next_cursor.get().is_some()
                                            }
                                        >
                                            <div class="note-list-footer">
                                                <Button
                                                    class="note-list-more"
                                                    variant=ButtonVariant::Ghost
                                                    disabled=Signal::derive(move || notes_loading_more.get())
                                                    on_click=Callback::new(move |_| {
                                                        let Some(cursor) = notes_next_cursor.get_untracked() else {
                                                            return;
                                                        };
                                                        if notes_loading_more.get_untracked() {
                                                            return;
                                                        }

                                                        let query = active_query.get_untracked();
                                                        let refresh = refresh_key.get_untracked();
                                                        set_notes_loading_more.set(true);
                                                        set_notes_error.set(None);

                                                        spawn_local(async move {
                                                            let result =
                                                                list_notes_page(query.clone(), Some(cursor)).await;
                                                            if active_query.get_untracked() != query
                                                                || refresh_key.get_untracked() != refresh
                                                            {
                                                                return;
                                                            }

                                                            set_notes_loading_more.set(false);

                                                            match result {
                                                                Ok(page) => {
                                                                    set_notes.update(|items| items.extend(page.notes));
                                                                    set_notes_next_cursor.set(page.next_cursor);
                                                                }
                                                                Err(error) => {
                                                                    set_notes_error.set(Some(error.to_string()));
                                                                }
                                                            }
                                                        });
                                                    })
                                                >
                                                    {move || {
                                                        if notes_loading_more.get() {
                                                            "正在加载更多…"
                                                        } else {
                                                            "加载更多"
                                                        }
                                                    }}
                                                </Button>
                                            </div>
                                        </Show>
                                    </div>
                                }
                                    .into_any()
                            }
                        }}
                    </div>
                </div>

                <div
                    class:is-resizing=move || is_resizing.get()
                    class="resizer"
                    on:mousedown=move |ev| {
                        ev.prevent_default();
                        set_is_resizing.set(true);
                    }
                ></div>

                <div class="main-content">
                    {move || match home_mode.get() {
                        HomeMode::Settings => view! { <SettingsPanel/> }.into_any(),
                        HomeMode::Empty => {
                            view! {
                                <div class="empty-state">
                                    "点击左上角 + 创建新笔记，或点击一条已有笔记开始查看"
                                </div>
                            }
                                .into_any()
                        }
                        HomeMode::Viewing(_) => {
                            view! {
                                <div class="editor-container">
                                    <div class="editor-header">
                                        <h2>{move || editor_title.get()}</h2>
                                        <p class="editor-subtitle">{move || editor_subtitle.get()}</p>
                                    </div>

                                    <NoteViewer note=active_note />

                                    <StatusBar status active_query />
                                </div>
                            }
                                .into_any()
                        }
                        HomeMode::Editing(_) | HomeMode::Creating(_) => {
                            view! {
                                <div class="editor-container">
                                    <div class="editor-header editor-header-editing">
                                        <div>
                                            <h2>{move || editor_title.get()}</h2>
                                            <p class="editor-subtitle">{move || editor_subtitle.get()}</p>
                                        </div>

                                        <EditorActions
                                            is_saving
                                            can_save
                                            is_dirty
                                            save_label
                                            on_save=Callback::new(move |_| {
                                                let Some(current_draft) = draft.get_untracked() else {
                                                    return;
                                                };
                                                let content = current_draft.content.trim().to_string();
                                                if content.is_empty() {
                                                    set_status.set("请输入笔记内容".to_string());
                                                    return;
                                                }

                                                let tags = current_draft.tags;
                                                let current_note = active_note.get_untracked();
                                                set_is_saving.set(true);

                                                spawn_local(async move {
                                                    let result = if let Some(note) = current_note.clone() {
                                                        edit_note_server(note.id.clone(), content.clone(), tags).await
                                                    } else {
                                                        create_note_server(content.clone(), tags).await
                                                    };

                                                    set_is_saving.set(false);

                                                    match result {
                                                        Ok(note) => {
                                                            let message = if current_note.is_some() {
                                                                "已生成新版本"
                                                            } else {
                                                                "已创建笔记"
                                                            };
                                                            set_home_mode.set(HomeMode::Viewing(note.clone()));
                                                            set_refresh_key.update(|value| *value += 1);
                                                            set_status.set(format!(
                                                                "{message} {}",
                                                                short_note_id(&note.id)
                                                            ));
                                                        }
                                                        Err(error) => {
                                                            set_status.set(format!("保存失败: {error}"));
                                                        }
                                                    }
                                                });
                                            })
                                            on_discard=Callback::new(move |_| {
                                                if let Some(note) = active_note.get_untracked() {
                                                    set_home_mode.set(HomeMode::Viewing(note));
                                                    set_status.set("已放弃当前修改".to_string());
                                                } else {
                                                    set_home_mode.set(HomeMode::Empty);
                                                    set_status.set("已放弃新建内容".to_string());
                                                }
                                            })
                                        />
                                    </div>

                                    <TagEditor
                                        tags=Memo::new(move |_| draft.get().map(|current| current.tags).unwrap_or_default())
                                        on_remove_tag=Callback::new(move |tag_name: String| {
                                            set_home_mode.update(|mode| match mode {
                                                HomeMode::Editing(session) => session.draft.remove_tag(&tag_name),
                                                HomeMode::Creating(draft) => draft.remove_tag(&tag_name),
                                                HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => {}
                                            });
                                        })
                                        on_add_tag=Callback::new(move |tag_value: String| {
                                            set_home_mode.update(|mode| match mode {
                                                HomeMode::Editing(session) => session.draft.add_tag(tag_value.clone()),
                                                HomeMode::Creating(draft) => draft.add_tag(tag_value.clone()),
                                                HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => {}
                                            });
                                        })
                                    />

                                    <TagSuggestions
                                        suggestions=available_recommended_tags
                                        on_add_tag=Callback::new(move |tag_value: String| {
                                            set_home_mode.update(|mode| match mode {
                                                HomeMode::Editing(session) => session.draft.add_tag(tag_value.clone()),
                                                HomeMode::Creating(draft) => draft.add_tag(tag_value.clone()),
                                                HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => {}
                                            });
                                        })
                                    />

                                    <textarea
                                        id="editor"
                                        placeholder="点此开始记录笔记"
                                        prop:value=move || draft.get().map(|current| current.content).unwrap_or_default()
                                        on:input=move |ev| {
                                            let value = event_target_value(&ev);
                                            set_home_mode.update(|mode| match mode {
                                                HomeMode::Editing(session) => session.draft.content = value.clone(),
                                                HomeMode::Creating(draft) => draft.content = value.clone(),
                                                HomeMode::Empty | HomeMode::Settings | HomeMode::Viewing(_) => {}
                                            });
                                        }
                                    />

                                    <StatusBar status active_query />
                                </div>
                            }
                                .into_any()
                        }
                    }}

                    <Show when=move || confirm_state.get().is_some()>
                        {move || match confirm_state.get() {
                            Some(ConfirmState::Discard(target)) => {
                                view! {
                                    <ConfirmDialog
                                        title="放弃未保存的修改？"
                                        description="当前内容还没有保存，离开后这些修改会丢失。"
                                        cancel_label="继续编辑"
                                        confirm_label="放弃改动并离开"
                                        on_cancel=Callback::new(move |_| set_confirm_state.set(None))
                                        on_confirm=Callback::new(move |_| {
                                            set_confirm_state.set(None);
                                            navigate.run(target.clone());
                                        })
                                    />
                                }
                                    .into_any()
                            }
                            Some(ConfirmState::Delete(note)) => {
                                let note_id = note.id.clone();
                                view! {
                                    <ConfirmDialog
                                        title="删除这条笔记？"
                                        description="删除后这条笔记会从当前列表中移除。"
                                        cancel_label="取消"
                                        confirm_label="确认删除"
                                        danger=true
                                        on_cancel=Callback::new(move |_| set_confirm_state.set(None))
                                        on_confirm=Callback::new(move |_| {
                                            let deleting_id = note_id.clone();
                                            set_confirm_state.set(None);
                                            set_is_deleting.set(true);

                                            spawn_local(async move {
                                                let result = delete_note_server(deleting_id.clone()).await;
                                                set_is_deleting.set(false);

                                                match result {
                                                    Ok(()) => {
                                                        if active_note.get_untracked().is_some_and(|current| current.id == deleting_id) {
                                                            set_home_mode.set(HomeMode::Empty);
                                                        }
                                                        set_refresh_key.update(|value| *value += 1);
                                                        set_status.set(format!(
                                                            "已删除 {}",
                                                            short_note_id(&deleting_id)
                                                        ));
                                                    }
                                                    Err(error) => {
                                                        set_status.set(format!("删除失败: {error}"));
                                                    }
                                                }
                                            });
                                        })
                                    />
                                }
                                    .into_any()
                            }
                            None => view! { <></> }.into_any(),
                        }}
                    </Show>
                </div>
            </div>
        </Transition>
    }
}
