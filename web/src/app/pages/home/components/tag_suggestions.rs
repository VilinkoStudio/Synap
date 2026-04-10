use leptos::prelude::*;

use crate::app::ui::Button;

#[component]
pub fn TagSuggestions(
    suggestions: Memo<Vec<String>>,
    on_add_tag: Callback<String>,
) -> impl IntoView {
    view! {
        <Transition fallback=move || view! { <></> }>
            {move || {
                let suggestions = suggestions.get();
                if suggestions.is_empty() {
                    view! { <></> }.into_any()
                } else {
                    view! {
                        <div class="tag-suggestions">
                            <span class="tag-suggestion-label">"推荐标签"</span>
                            {suggestions
                                .into_iter()
                                .map(|tag| {
                                    let tag_value = tag.clone();
                                    view! {
                                        <Button
                                            class="tag-suggestion"
                                            on_click=Callback::new(move |_| on_add_tag.run(tag_value.clone()))
                                        >
                                            {tag}
                                        </Button>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }
            }}
        </Transition>
    }
}
