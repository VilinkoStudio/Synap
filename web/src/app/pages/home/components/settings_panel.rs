use leptos::prelude::*;

#[component]
pub fn SettingsPanel() -> impl IntoView {
    view! {
        <div class="settings-container">
            <h2>"设置"</h2>

            <h3>"备份和恢复"</h3>
            <div class="setting-card">
                <div class="setting-info">
                    <h3>"数据备份"</h3>
                    <p>"直接下载当前使用中的 redb 数据库文件。"</p>
                </div>
                <form action="/api/settings/export" method="get">
                    <button class="ui-button" type="submit">
                        "导出备份"
                    </button>
                </form>
            </div>

            <form class="setting-card setting-form" action="/api/settings/import" method="post" enctype="multipart/form-data">
                <div class="setting-info">
                    <h3>"恢复数据"</h3>
                    <p>"上传 redb 数据库文件并替换当前数据库，页面会在导入后刷新。"</p>
                </div>
                <div class="setting-actions">
                    <input
                        class="settings-file-input"
                        type="file"
                        name="database"
                        accept=".redb,application/octet-stream"
                    />
                    <button class="ui-button ui-button--primary" type="submit">
                        "导入数据库"
                    </button>
                </div>
            </form>

            <h3 class="settings-about-title">"关于"</h3>
            <div class="settings-about-block">
                <h2>"Synap"</h2>
                <span class="subtitle-text">"Leptos 重构版"</span>
                <span class="main-text">
                    "一个基于有向无环图（DAG）的极简思维捕获与路由中枢。当前设置页提供本地 redb 数据库的直接导入与导出。"
                </span>
            </div>
        </div>
    }
}
