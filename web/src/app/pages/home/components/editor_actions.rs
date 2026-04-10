use leptos::prelude::*;

use crate::app::ui::{Button, ButtonVariant};

#[component]
pub fn EditorActions(
    is_saving: ReadSignal<bool>,
    can_save: Signal<bool>,
    is_dirty: Signal<bool>,
    save_label: Memo<String>,
    on_save: Callback<()>,
    on_discard: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="editor-actions">
            <Show when=move || is_dirty.get()>
                <span class="editor-dirty-indicator">"未保存更改"</span>
            </Show>

            <Button
                class="editor-action editor-action-secondary"
                variant=ButtonVariant::Ghost
                on_click=on_discard
            >
                "放弃修改"
            </Button>

            <Button
                class="editor-action editor-action-primary"
                variant=ButtonVariant::Primary
                disabled=Signal::derive(move || !can_save.get())
                on_click=on_save
            >
                {move || {
                    if is_saving.get() {
                        "正在保存...".to_string()
                    } else {
                        save_label.get()
                    }
                }}
            </Button>
        </div>
    }
}
