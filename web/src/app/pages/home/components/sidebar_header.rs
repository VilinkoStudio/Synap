use leptos::prelude::*;

use crate::app::ui::IconButton;

#[component]
pub fn SidebarHeader(
    #[prop(into)] title: String,
    on_refresh: Callback<()>,
    on_create: Callback<()>,
    on_open_settings: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="sidebar-header">
            <span>{title}</span>
            <div class="header-actions">
                <IconButton title="设置" on_click=on_open_settings>
                    "⚙"
                </IconButton>
                <IconButton title="刷新" on_click=on_refresh>
                    "↻"
                </IconButton>
                <IconButton title="新建笔记" on_click=on_create>
                    "+"
                </IconButton>
            </div>
        </div>
    }
}
