use leptos::{html, prelude::*};
use synap_core::NoteDTO;

use crate::app::ui::IconButton;
use crate::app::utils::{format_timestamp, preview};

#[component]
pub fn NoteList(
    notes: Signal<Vec<NoteDTO>>,
    active_note: Memo<Option<NoteDTO>>,
    on_select: Callback<NoteDTO>,
    on_edit: Callback<NoteDTO>,
    on_delete: Callback<NoteDTO>,
    actions_disabled: Signal<bool>,
) -> impl IntoView {
    let list_ref = NodeRef::<html::Ul>::new();
    let indicator_ref = NodeRef::<html::Div>::new();

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let active_id = active_note.get().map(|note| note.id);
        let _ = notes.get();

        request_animation_frame(move || {
            use wasm_bindgen::JsCast;

            let Some(indicator) = indicator_ref.get() else {
                return;
            };
            let Some(list) = list_ref.get() else {
                return;
            };
            let indicator = indicator.unchecked_into::<web_sys::HtmlElement>();

            let style = indicator.style();

            let Some(active_id) = active_id else {
                let _ = style.set_property("opacity", "0");
                return;
            };

            let selector = format!(r#"[data-note-id="{active_id}"]"#);
            let Ok(Some(active_item)) = list.query_selector(&selector) else {
                let _ = style.set_property("opacity", "0");
                return;
            };
            let Ok(active_item) = active_item.dyn_into::<web_sys::HtmlElement>() else {
                let _ = style.set_property("opacity", "0");
                return;
            };

            let offset_top = active_item.offset_top();
            let item_height = active_item.offset_height();
            let indicator_height = 16_i32;
            let y = offset_top + ((item_height - indicator_height) / 2);

            let _ = style.set_property("transform", &format!("translateY({y}px)"));
            let _ = style.set_property("opacity", "1");
        });
    });

    view! {
        <ul class="note-list" node_ref=list_ref>
            <div class="active-indicator" node_ref=indicator_ref></div>
            <For
                each=move || notes.get()
                key=|note| note.id.clone()
                children=move |note| {
                    let note_for_click = note.clone();
                    let note_for_edit = note.clone();
                    let note_for_delete = note.clone();
                    let note_id = note.id.clone();
                    let content = preview(&note.content, 96);
                    let tags = note.tags.clone();

                    view! {
                        <li
                            data-note-id=note.id.clone()
                            class=move || {
                                if active_note.get().is_some_and(|active| active.id == note_id) {
                                    "note-item active"
                                } else {
                                    "note-item"
                                }
                            }
                            on:click=move |_| on_select.run(note_for_click.clone())
                        >
                            <div class="note-item-actions">
                                <IconButton
                                    class="note-item-action"
                                    title="编辑笔记"
                                    disabled=Signal::derive(move || actions_disabled.get())
                                    stop_propagation=true
                                    on_click=Callback::new(move |_| on_edit.run(note_for_edit.clone()))
                                >
                                    "✎"
                                </IconButton>
                                <IconButton
                                    class="note-item-action note-item-action-danger"
                                    title="删除笔记"
                                    disabled=Signal::derive(move || actions_disabled.get())
                                    stop_propagation=true
                                    on_click=Callback::new(move |_| on_delete.run(note_for_delete.clone()))
                                >
                                    "⌫"
                                </IconButton>
                            </div>
                            <div class="note-content">{content}</div>

                            <div class="note-meta">
                                <span class="note-time">{format_timestamp(note.created_at)}</span>
                                {tags
                                    .into_iter()
                                    .map(|tag| view! { <span class="note-meta-tag">{tag}</span> })
                                    .collect_view()}
                            </div>
                        </li>
                    }
                }
            />
        </ul>
    }
}
