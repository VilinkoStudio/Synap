use leptos::prelude::*;

#[component]
pub fn StatusBar(status: ReadSignal<String>, active_query: ReadSignal<String>) -> impl IntoView {
    view! {
        <div class="status-bar">
            <span>{move || status.get()}</span>
            <span class="subtitle-text">
                {move || {
                    if active_query.get().is_empty() {
                        "数据源：最近笔记".to_string()
                    } else {
                        format!("当前检索：{}", active_query.get())
                    }
                }}
            </span>
        </div>
    }
}
