use leptos::{prelude::*, task::spawn_local};
use synap_core::{
    dto::{SyncSessionRecordDTO, SyncSessionRoleDTO},
    PeerTrustStatusDTO, SyncStatusDTO,
};

use crate::app::{
    server::{
        approve_peer_server, delete_peer_server, ensure_sync_listener_server,
        get_sync_overview_server, set_peer_status_server, update_peer_note_server, WebPeerDTO,
        WebSyncOverviewDTO,
    },
    utils::format_timestamp,
};

#[component]
pub fn SettingsPanel() -> impl IntoView {
    let (sync_refresh_key, set_sync_refresh_key) = signal(0_u64);
    let (sync_message, set_sync_message) = signal(None::<String>);
    let (is_sync_mutating, set_is_sync_mutating) = signal(false);
    let (active_peer_id, set_active_peer_id) = signal(None::<String>);
    let (peer_note_draft, set_peer_note_draft) = signal(String::new());
    let (selected_session_id, set_selected_session_id) = signal(None::<String>);

    let sync_overview = Resource::new(
        move || sync_refresh_key.get(),
        |_| async move { get_sync_overview_server().await },
    );

    let active_peer = Memo::new(move |_| {
        let selected = active_peer_id.get()?;
        let overview = sync_overview.get()?;
        let overview = overview.ok()?;
        overview.peers.into_iter().find(|peer| peer.id == selected)
    });

    let selected_session = Memo::new(move |_| {
        let selected = selected_session_id.get()?;
        let overview = sync_overview.get()?;
        let overview = overview.ok()?;
        overview
            .recent_sync_sessions
            .into_iter()
            .find(|session| session.id == selected)
    });

    let refresh_sync = Callback::new(move |_| {
        set_sync_refresh_key.update(|value| *value += 1);
    });

    let run_sync_action = move |future: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, String>>>,
    >| {
        set_is_sync_mutating.set(true);
        spawn_local(async move {
            let result = future.await;
            set_is_sync_mutating.set(false);
            match result {
                Ok(message) => {
                    set_sync_message.set(Some(message));
                    set_sync_refresh_key.update(|value| *value += 1);
                }
                Err(error) => {
                    set_sync_message.set(Some(error));
                }
            }
        });
    };

    view! {
        <div class="settings-container">
            <h2>"设置"</h2>

            <h3>"同步与信任"</h3>
            <div class="setting-card setting-card--stacked">
                <div class="setting-info">
                    <h3>"SSR 同步监听"</h3>
                    <p>"Web 端作为常驻同步节点运行。其他设备主要通过当前服务端地址和端口连接到这里。"</p>
                </div>
                <div class="setting-actions">
                    <button
                        class="ui-button ui-button--primary"
                        type="button"
                        disabled=move || is_sync_mutating.get()
                        on:click=move |_| {
                            run_sync_action(Box::pin(async move {
                                ensure_sync_listener_server()
                                    .await
                                    .map(|state| {
                                        format!(
                                            "监听状态已刷新：{}{}",
                                            state.status,
                                            state
                                                .listen_port
                                                .map(|port| format!(" · 端口 {port}"))
                                                .unwrap_or_default()
                                        )
                                    })
                                    .map_err(|error| format!("刷新监听状态失败: {error}"))
                            }));
                        }
                    >
                        {move || if is_sync_mutating.get() { "处理中…" } else { "刷新监听状态" }}
                    </button>
                </div>
            </div>

            <Suspense fallback=move || view! { <div class="setting-card">"正在读取同步状态…"</div> }>
                {move || {
                    match sync_overview.get() {
                        Some(Ok(overview)) => view! {
                            <div class="sync-settings-grid">
                                <ListenerOverviewCard overview=overview.clone() />
                                <IdentityOverviewCard overview=overview.clone() />
                            </div>

                            {if let Some(message) = sync_message.get() {
                                view! { <div class="settings-inline-message">{message}</div> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}

                            <PeerManagementSection
                                overview=overview.clone()
                                active_peer=active_peer.get()
                                peer_note_draft
                                set_active_peer_id
                                set_peer_note_draft
                                is_sync_mutating
                                on_refresh=refresh_sync
                                on_run_action=Callback::new(move |action: SyncPeerAction| {
                                    match action {
                                        SyncPeerAction::Open(peer) => {
                                            set_active_peer_id.set(Some(peer.id));
                                            set_peer_note_draft.set(peer.note.unwrap_or_default());
                                        }
                                        SyncPeerAction::Approve(peer_id, note) => {
                                            run_sync_action(Box::pin(async move {
                                                approve_peer_server(peer_id, Some(note))
                                                    .await
                                                    .map(|_| "设备已设为可信".to_string())
                                                    .map_err(|error| format!("设为可信失败: {error}"))
                                            }));
                                        }
                                        SyncPeerAction::UpdateNote(peer_id, note) => {
                                            run_sync_action(Box::pin(async move {
                                                update_peer_note_server(peer_id, Some(note))
                                                    .await
                                                    .map(|_| "设备备注已更新".to_string())
                                                    .map_err(|error| format!("更新备注失败: {error}"))
                                            }));
                                        }
                                        SyncPeerAction::SetStatus(peer_id, status, label) => {
                                            run_sync_action(Box::pin(async move {
                                                set_peer_status_server(peer_id, status)
                                                    .await
                                                    .map(|_| format!("设备状态已更新为{label}"))
                                                    .map_err(|error| format!("更新设备状态失败: {error}"))
                                            }));
                                        }
                                        SyncPeerAction::Delete(peer_id) => {
                                            run_sync_action(Box::pin(async move {
                                                delete_peer_server(peer_id)
                                                    .await
                                                    .map(|_| "设备记录已删除".to_string())
                                                    .map_err(|error| format!("删除设备失败: {error}"))
                                            }));
                                        }
                                    }
                                })
                            />

                            <SyncHistorySection
                                sessions=overview.recent_sync_sessions
                                selected_session=selected_session.get()
                                on_select=Callback::new(move |session_id: String| {
                                    set_selected_session_id.set(Some(session_id));
                                })
                            />
                        }
                            .into_any(),
                        Some(Err(error)) => view! {
                            <div class="setting-card">
                                <div class="setting-info">
                                    <h3>"同步状态读取失败"</h3>
                                    <p>{error.to_string()}</p>
                                </div>
                                <button
                                    class="ui-button"
                                    type="button"
                                    on:click=move |_| refresh_sync.run(())
                                >
                                    "重试"
                                </button>
                            </div>
                        }
                            .into_any(),
                        None => view! { <div class="setting-card">"正在读取同步状态…"</div> }.into_any(),
                    }
                }}
            </Suspense>

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

#[derive(Clone)]
enum SyncPeerAction {
    Open(WebPeerDTO),
    Approve(String, String),
    UpdateNote(String, String),
    SetStatus(String, PeerTrustStatusDTO, &'static str),
    Delete(String),
}

#[component]
fn ListenerOverviewCard(overview: WebSyncOverviewDTO) -> impl IntoView {
    let listener = overview.listener;
    view! {
        <div class="setting-card setting-card--stacked sync-overview-card">
            <div class="sync-card-header">
                <div>
                    <div class="sync-eyebrow">"监听状态"</div>
                    <h3>{listener.status.clone()}</h3>
                </div>
                <div class="sync-chip" class:sync-chip--active=listener.is_listening>
                    {if listener.is_listening { "Listening" } else { "Stopped" }}
                </div>
            </div>
            <div class="sync-meta-grid">
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"协议"</span>
                    <strong>{listener.protocol}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"后端"</span>
                    <strong>{listener.backend}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"端口"</span>
                    <strong>{listener.listen_port.map(|port| port.to_string()).unwrap_or_else(|| "未分配".to_string())}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"局域网地址"</span>
                    <strong>
                        {if listener.local_addresses.is_empty() {
                            "未检测到".to_string()
                        } else {
                            listener.local_addresses.join(" · ")
                        }}
                    </strong>
                </div>
            </div>
            {listener.error_message.map(|message| {
                view! { <div class="settings-inline-error">{message}</div> }
            })}
        </div>
    }
}

#[component]
fn IdentityOverviewCard(overview: WebSyncOverviewDTO) -> impl IntoView {
    let identity = overview.local_identity;
    let cards = vec![
        (
            "通信身份",
            identity.identity.kaomoji_fingerprint,
            identity.identity.display_public_key_base64,
        ),
        (
            "签名身份",
            identity.signing.kaomoji_fingerprint,
            identity.signing.display_public_key_base64,
        ),
    ];

    view! {
        <div class="setting-card setting-card--stacked sync-overview-card">
            <div class="sync-card-header">
                <div>
                    <div class="sync-eyebrow">"本机身份"</div>
                    <h3>"用于同步握手与设备识别"</h3>
                </div>
            </div>
            <div class="identity-grid">
                {cards
                    .into_iter()
                    .map(|(title, kaomoji, key)| {
                        view! {
                            <div class="identity-panel">
                                <div class="identity-panel-head">
                                    <div class="sync-chip sync-chip--soft">{kaomoji}</div>
                                    <strong>{title}</strong>
                                </div>
                                <code class="identity-public-key">{key}</code>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
fn PeerManagementSection(
    overview: WebSyncOverviewDTO,
    active_peer: Option<WebPeerDTO>,
    peer_note_draft: ReadSignal<String>,
    set_active_peer_id: WriteSignal<Option<String>>,
    set_peer_note_draft: WriteSignal<String>,
    is_sync_mutating: ReadSignal<bool>,
    on_refresh: Callback<()>,
    on_run_action: Callback<SyncPeerAction>,
) -> impl IntoView {
    let peers = overview.peers;
    let pending_count = peers
        .iter()
        .filter(|peer| peer.status == PeerTrustStatusDTO::Pending)
        .count();

    view! {
        <div class="settings-section-block">
            <div class="settings-section-heading">
                <div>
                    <h3>"设备信任"</h3>
                    <p>{format!("共 {} 台设备，待处理 {} 台。", peers.len(), pending_count)}</p>
                </div>
                <button class="ui-button" type="button" on:click=move |_| on_refresh.run(())>
                    "刷新列表"
                </button>
            </div>

            <div class="sync-peer-list">
                {if peers.is_empty() {
                    view! {
                        <div class="setting-card">
                            <div class="setting-info">
                                <h3>"还没有发现设备"</h3>
                                <p>"当有设备尝试与当前 Web 节点同步后，这里会出现待处理或已信任的设备记录。"</p>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    peers
                        .into_iter()
                        .map(|peer| {
                            let peer_for_open = peer.clone();
                            let peer_id = peer.id.clone();
                            let is_pending = peer.status == PeerTrustStatusDTO::Pending;
                            view! {
                                <div class="setting-card setting-card--stacked sync-peer-card">
                                    <div class="sync-card-header">
                                        <div>
                                            <div class="sync-eyebrow">
                                                {match peer.status {
                                                    PeerTrustStatusDTO::Pending => "待信任",
                                                    PeerTrustStatusDTO::Trusted => "已信任",
                                                    PeerTrustStatusDTO::Retired => "已停用",
                                                    PeerTrustStatusDTO::Revoked => "已撤销",
                                                }}
                                            </div>
                                            <h3>{peer.note.clone().unwrap_or_else(|| "未命名设备".to_string())}</h3>
                                        </div>
                                        <div class="sync-chip">{peer.kaomoji_fingerprint.clone()}</div>
                                    </div>

                                    <div class="sync-peer-summary">
                                        <span>{format!("{} · {}", peer.algorithm, peer.display_public_key_base64)}</span>
                                    </div>

                                    <div class="sync-peer-actions">
                                        <button
                                            class="ui-button"
                                            type="button"
                                            on:click=move |_| {
                                                on_run_action.run(SyncPeerAction::Open(peer_for_open.clone()));
                                            }
                                        >
                                            "管理"
                                        </button>

                                        {if is_pending {
                                            view! {
                                                <button
                                                    class="ui-button ui-button--primary"
                                                    type="button"
                                                    disabled=move || is_sync_mutating.get()
                                                    on:click=move |_| {
                                                        on_run_action.run(SyncPeerAction::Approve(
                                                            peer_id.clone(),
                                                            String::new(),
                                                        ));
                                                    }
                                                >
                                                    "直接信任"
                                                </button>
                                            }
                                                .into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }}
                                    </div>
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>

            {active_peer.map(|peer| {
                let peer_id_for_note = peer.id.clone();
                let peer_id_for_approve = peer.id.clone();
                let peer_id_for_pending = peer.id.clone();
                let peer_id_for_retired = peer.id.clone();
                let peer_id_for_revoked = peer.id.clone();
                let peer_id_for_delete = peer.id.clone();
                view! {
                    <div class="setting-card setting-card--stacked sync-peer-detail-card">
                        <div class="sync-card-header">
                            <div>
                                <div class="sync-eyebrow">"设备详情"</div>
                                <h3>{peer.note.clone().unwrap_or_else(|| "未命名设备".to_string())}</h3>
                            </div>
                            <button
                                class="ui-button ui-button--ghost"
                                type="button"
                                on:click=move |_| set_active_peer_id.set(None)
                            >
                                "收起"
                            </button>
                        </div>

                        <div class="sync-peer-detail-grid">
                            <div class="sync-meta-item">
                                <span class="sync-meta-label">"状态"</span>
                                <strong>{match peer.status {
                                    PeerTrustStatusDTO::Pending => "待信任",
                                    PeerTrustStatusDTO::Trusted => "已信任",
                                    PeerTrustStatusDTO::Retired => "已停用",
                                    PeerTrustStatusDTO::Revoked => "已撤销",
                                }}</strong>
                            </div>
                            <div class="sync-meta-item">
                                <span class="sync-meta-label">"识别码"</span>
                                <strong>{peer.kaomoji_fingerprint.clone()}</strong>
                            </div>
                        </div>

                        <label class="settings-field">
                            <span>"备注"</span>
                            <input
                                class="settings-input"
                                prop:value=move || peer_note_draft.get()
                                on:input=move |ev| set_peer_note_draft.set(event_target_value(&ev))
                            />
                        </label>

                        <div class="sync-peer-summary sync-peer-summary--full">
                            <span>{peer.display_public_key_base64.clone()}</span>
                        </div>

                        <div class="sync-peer-actions sync-peer-actions--detail">
                            <button
                                class="ui-button"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::UpdateNote(
                                        peer_id_for_note.clone(),
                                        peer_note_draft.get_untracked(),
                                    ));
                                }
                            >
                                "保存备注"
                            </button>
                            <button
                                class="ui-button ui-button--primary"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::Approve(
                                        peer_id_for_approve.clone(),
                                        peer_note_draft.get_untracked(),
                                    ));
                                }
                            >
                                "设为可信"
                            </button>
                            <button
                                class="ui-button"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::SetStatus(
                                        peer_id_for_pending.clone(),
                                        PeerTrustStatusDTO::Pending,
                                        "待信任",
                                    ));
                                }
                            >
                                "设为待处理"
                            </button>
                            <button
                                class="ui-button"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::SetStatus(
                                        peer_id_for_retired.clone(),
                                        PeerTrustStatusDTO::Retired,
                                        "已停用",
                                    ));
                                }
                            >
                                "停用"
                            </button>
                            <button
                                class="ui-button"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::SetStatus(
                                        peer_id_for_revoked.clone(),
                                        PeerTrustStatusDTO::Revoked,
                                        "已撤销",
                                    ));
                                }
                            >
                                "撤销"
                            </button>
                            <button
                                class="ui-button"
                                type="button"
                                disabled=move || is_sync_mutating.get()
                                on:click=move |_| {
                                    on_run_action.run(SyncPeerAction::Delete(peer_id_for_delete.clone()));
                                    set_active_peer_id.set(None);
                                }
                            >
                                "删除记录"
                            </button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

#[component]
fn SyncHistorySection(
    sessions: Vec<SyncSessionRecordDTO>,
    selected_session: Option<SyncSessionRecordDTO>,
    on_select: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="settings-section-block">
            <div class="settings-section-heading">
                <div>
                    <h3>"最近同步"</h3>
                    <p>"点击列表项查看本次同步的细节。"</p>
                </div>
            </div>

            <div class="sync-session-list">
                {if sessions.is_empty() {
                    view! {
                        <div class="setting-card">
                            <div class="setting-info">
                                <h3>"还没有同步记录"</h3>
                                <p>"当其他设备与当前 Web 节点完成握手并进入同步流程后，这里会出现统计信息。"</p>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    sessions
                        .iter()
                        .map(|session| {
                            let session_id = session.id.clone();
                            view! {
                                <button
                                    class="setting-card setting-card--stacked sync-session-card"
                                    type="button"
                                    on:click=move |_| on_select.run(session_id.clone())
                                >
                                    <div class="sync-card-header">
                                        <div>
                                            <div class="sync-eyebrow">
                                                {match session.role {
                                                    SyncSessionRoleDTO::Initiator => "主动同步",
                                                    SyncSessionRoleDTO::Listener => "监听接入",
                                                    SyncSessionRoleDTO::RelayFetch => "Relay 拉取",
                                                    SyncSessionRoleDTO::RelayPush => "Relay 推送",
                                                }}
                                            </div>
                                            <h3>{session.peer_label.clone().unwrap_or_else(|| "未命名设备".to_string())}</h3>
                                        </div>
                                        <div class="sync-chip">
                                            {match session.status {
                                                SyncStatusDTO::Completed => "完成",
                                                SyncStatusDTO::PendingTrust => "待信任",
                                                SyncStatusDTO::Failed => "失败",
                                            }}
                                        </div>
                                    </div>
                                    <div class="sync-session-meta">
                                        <span>{format_timestamp(session.finished_at_ms)}</span>
                                        <span>{format!("收 {} / 发 {}", session.records_received, session.records_sent)}</span>
                                        <span>{format!("{} ms", session.duration_ms)}</span>
                                    </div>
                                </button>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>

            {selected_session.map(|session| {
                view! { <SyncSessionDetailCard session/> }
            })}
        </div>
    }
}

#[component]
fn SyncSessionDetailCard(session: SyncSessionRecordDTO) -> impl IntoView {
    view! {
        <div class="setting-card setting-card--stacked sync-session-detail-card">
            <div class="sync-card-header">
                <div>
                    <div class="sync-eyebrow">"同步详情"</div>
                    <h3>{session.peer_label.clone().unwrap_or_else(|| "未命名设备".to_string())}</h3>
                </div>
                <div class="sync-chip">
                    {match session.status {
                        SyncStatusDTO::Completed => "完成",
                        SyncStatusDTO::PendingTrust => "待信任",
                        SyncStatusDTO::Failed => "失败",
                    }}
                </div>
            </div>

            <div class="sync-meta-grid">
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"开始时间"</span>
                    <strong>{format_timestamp(session.started_at_ms)}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"结束时间"</span>
                    <strong>{format_timestamp(session.finished_at_ms)}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"发送记录"</span>
                    <strong>{session.records_sent.to_string()}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"接收记录"</span>
                    <strong>{session.records_received.to_string()}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"应用记录"</span>
                    <strong>{session.records_applied.to_string()}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"跳过记录"</span>
                    <strong>{session.records_skipped.to_string()}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"发送字节"</span>
                    <strong>{session.bytes_sent.to_string()}</strong>
                </div>
                <div class="sync-meta-item">
                    <span class="sync-meta-label">"接收字节"</span>
                    <strong>{session.bytes_received.to_string()}</strong>
                </div>
            </div>

            {session.error_message.map(|message| {
                view! { <div class="settings-inline-error">{message}</div> }
            })}
        </div>
    }
}
