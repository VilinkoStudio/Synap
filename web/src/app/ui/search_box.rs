use leptos::prelude::*;

use crate::app::ui::IconButton;

const DEFAULT_DEBOUNCE_MS: i32 = 300;

#[component]
pub fn SearchBox(
    search_input: ReadSignal<String>,
    set_search_input: WriteSignal<String>,
    on_search: Callback<()>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional)] debounce_ms: Option<i32>,
) -> impl IntoView {
    let placeholder = placeholder.unwrap_or_else(|| "搜索笔记或标签...".to_string());
    #[cfg_attr(not(feature = "hydrate"), allow(unused_variables))]
    let debounce_ms = debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS);

    #[cfg(feature = "hydrate")]
    let debounce_handle = RwSignal::new(None::<i32>);

    let clear_pending_search = Callback::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(handle) = debounce_handle.get_untracked() {
                if let Some(window) = web_sys::window() {
                    window.clear_timeout_with_handle(handle);
                }
                debounce_handle.set(None);
            }
        }
    });

    let run_search = {
        let clear_pending_search = clear_pending_search.clone();
        let on_search = on_search.clone();
        Callback::new(move |_| {
            clear_pending_search.run(());
            on_search.run(());
        })
    };

    let schedule_search = {
        #[cfg_attr(not(feature = "hydrate"), allow(unused_variables))]
        let clear_pending_search = clear_pending_search.clone();
        let on_search = on_search.clone();
        move || {
            #[cfg(feature = "hydrate")]
            {
                use wasm_bindgen::JsCast;

                clear_pending_search.run(());

                let Some(window) = web_sys::window() else {
                    on_search.run(());
                    return;
                };

                let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
                    on_search.run(());
                });

                match window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.unchecked_ref(),
                    debounce_ms,
                ) {
                    Ok(handle) => debounce_handle.set(Some(handle)),
                    Err(_) => on_search.run(()),
                }
            }

            #[cfg(not(feature = "hydrate"))]
            {
                on_search.run(());
            }
        }
    };

    view! {
        <div class="search-container">
            <div class="search-box">
                <input
                    type="text"
                    class="search-input"
                    placeholder=placeholder
                    prop:value=move || search_input.get()
                    on:input=move |ev| {
                        set_search_input.set(event_target_value(&ev));
                        schedule_search();
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            run_search.run(());
                        }
                    }
                />
                <Show when=move || !search_input.get().trim().is_empty()>
                    <IconButton
                        class="search-clear-btn"
                        title="清空搜索"
                        on_click=Callback::new(move |_| {
                            set_search_input.set(String::new());
                            run_search.run(());
                        })
                    >
                        "×"
                    </IconButton>
                </Show>
                <IconButton
                    class="search-btn"
                    title="搜索"
                    on_click=Callback::new(move |_| run_search.run(()))
                >
                    "⌕"
                </IconButton>
            </div>
        </div>
    }
}
