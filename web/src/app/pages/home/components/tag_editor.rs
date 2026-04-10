use leptos::prelude::*;

use crate::app::ui::IconButton;

#[component]
pub fn TagEditor(
    tags: Memo<Vec<String>>,
    on_remove_tag: Callback<String>,
    on_add_tag: Callback<String>,
) -> impl IntoView {
    let (input_value, set_input_value) = signal(String::new());

    view! {
        <div class="tag-bar">
            {move || {
                tags.get()
                    .into_iter()
                    .map(|tag| {
                        let tag_name = tag.clone();
                        view! {
                            <span class="tag-pill">
                                {tag}
                                <IconButton
                                    class="tag-delete"
                                    title="删除标签"
                                    on_click=Callback::new(move |_| on_remove_tag.run(tag_name.clone()))
                                >
                                    "×"
                                </IconButton>
                            </span>
                        }
                    })
                    .collect_view()
            }}
            <input
                type="text"
                class="tag-input"
                placeholder="输入标签后回车"
                prop:value=input_value
                on:input=move |ev| set_input_value.set(event_target_value(&ev))
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        ev.prevent_default();

                        let next_tag = input_value.get_untracked().trim().to_string();
                        if next_tag.is_empty() {
                            return;
                        }

                        on_add_tag.run(next_tag);
                        set_input_value.set(String::new());
                    }
                }
            />
        </div>
    }
}
