<svelte:options runes={false} />

<script lang="ts">
  import { afterUpdate, onMount, tick } from 'svelte';
  import '../styles/app.scss';
  import {
    approvePeerRemote,
    createNoteRemote,
    deleteNoteRemote,
    deletePeerRemote,
    editNoteRemote,
    getHome,
    getTagRecommendations,
    setPeerStatusRemote,
    updatePeerNoteRemote
  } from '$lib/remote/synap.remote';
  import type { Note, Peer, SyncOverview, TimelineNotesPage } from '$lib/types';

  type Mode = 'empty' | 'view' | 'create' | 'edit' | 'settings';
  type PendingAction =
    | { type: 'view'; note: Note }
    | { type: 'edit'; note: Note }
    | { type: 'create' }
    | { type: 'settings' };
  type ConfirmState =
    | { type: 'discard'; action: PendingAction }
    | { type: 'delete'; note: Note };

  const DEFAULT_SIDEBAR_WIDTH = 320;
  const MIN_SIDEBAR_WIDTH = 320;
  const MAIN_CONTENT_MIN_WIDTH = 400;

  let searchInput = '';
  let activeQuery = '';
  let mode: Mode = 'empty';
  let selectedNoteId = '';
  let draftContent = '';
  let draftTags: string[] = [];
  let statusMessage = 'Synap Web 已连接到 synap-core';
  let notesPage: TimelineNotesPage = { notes: [] };
  let syncOverview: SyncOverview | undefined;
  let notesLoading = true;
  let notesLoadingMore = false;
  let notesError = '';
  let isSaving = false;
  let isDeleting = false;
  let sidebarWidth = DEFAULT_SIDEBAR_WIDTH;
  let isResizing = false;
  let confirmState: ConfirmState | undefined;
  let recommendedTags: string[] = [];
  let recommendTimer: ReturnType<typeof setTimeout> | undefined;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let activePeerId = '';
  let peerNoteDraft = '';
  let syncMessage = '';
  let isSyncMutating = false;
  let selectedSessionId = '';
  let listElement: HTMLUListElement;
  let indicatorElement: HTMLDivElement;

  $: notes = notesPage.notes;
  $: activeNote = notes.find((note) => note.id === selectedNoteId);
  $: peers = syncOverview?.peers ?? [];
  $: activePeer = peers.find((peer) => peer.id === activePeerId);
  $: selectedSession = syncOverview?.recentSyncSessions.find((session) => session.id === selectedSessionId);
  $: isDirty =
    (mode === 'create' && (draftContent.trim() !== '' || draftTags.length > 0)) ||
    (mode === 'edit' &&
      !!activeNote &&
      (draftContent !== activeNote.content || draftTags.join('\u0000') !== activeNote.tags.join('\u0000')));
  $: isMutating = mode === 'create' || mode === 'edit';
  $: canSave = isMutating && !isSaving && !isDeleting && draftContent.trim() !== '';
  $: editorTitle =
    mode === 'edit'
      ? '编辑笔记'
      : mode === 'create'
        ? '新建笔记'
        : mode === 'settings'
          ? '设置'
          : mode === 'view'
            ? '查看笔记'
            : '笔记';
  $: editorSubtitle =
    mode === 'create'
      ? '写完后手动保存，才会生成新的笔记版本。'
      : mode === 'settings'
        ? '调整数据与界面设置'
        : activeNote
          ? `${shortNoteId(activeNote.id)} · ${formatTimestamp(activeNote.created_at)}`
          : '还没有可显示的笔记';
  $: availableRecommendedTags = recommendedTags.filter((tag) => !draftTags.includes(tag));

  onMount(() => {
    const savedWidth = Number(localStorage.getItem('synap_sidebar_width'));
    if (Number.isFinite(savedWidth) && savedWidth >= MIN_SIDEBAR_WIDTH) {
      sidebarWidth = clampSidebarWidth(window.innerWidth, savedWidth);
    }
    void loadNotes({ message: '已载入最近笔记' });
  });

  afterUpdate(() => {
    void updateActiveIndicator();
  });

  function shortNoteId(id: string) {
    return id.slice(0, 8);
  }

  function preview(content: string, limit = 96) {
    const flattened = content.replace(/\n/g, ' ');
    return flattened.length > limit ? `${flattened.slice(0, limit)}…` : flattened;
  }

  function formatTimestamp(value: bigint | number) {
    const timestamp = Number(value);
    const millis = timestamp > 10_000_000_000 ? timestamp : timestamp * 1000;
    const date = new Date(millis);
    if (Number.isNaN(date.getTime())) return String(value);
    const pad = (next: number) => String(next).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function clampSidebarWidth(windowWidth: number, width: number) {
    const maxWidth = Math.max(windowWidth - MAIN_CONTENT_MIN_WIDTH, MIN_SIDEBAR_WIDTH);
    return Math.min(Math.max(width, MIN_SIDEBAR_WIDTH), maxWidth);
  }

  async function updateActiveIndicator() {
    await tick();
    if (!listElement || !indicatorElement || !selectedNoteId) {
      if (indicatorElement) indicatorElement.style.opacity = '0';
      return;
    }
    const item = listElement.querySelector<HTMLElement>(`[data-note-id="${CSS.escape(selectedNoteId)}"]`);
    if (!item) {
      indicatorElement.style.opacity = '0';
      return;
    }
    const y = item.offsetTop + Math.max(0, Math.floor((item.offsetHeight - 16) / 2));
    indicatorElement.style.transform = `translateY(${y}px)`;
    indicatorElement.style.opacity = '1';
  }

  async function loadNotes(options: { cursor?: string; append?: boolean; message?: string } = {}) {
    const requestQuery = activeQuery;
    if (options.append) {
      notesLoadingMore = true;
    } else {
      notesLoading = true;
      notesError = '';
      notesPage = { notes: [] };
    }

    try {
      const home = await getHome({ query: requestQuery, cursor: options.cursor });
      if (requestQuery !== activeQuery) return;
      syncOverview = home.syncOverview;
      notesPage = options.append
        ? {
            notes: [...notesPage.notes, ...home.notesPage.notes],
            next_cursor: home.notesPage.next_cursor
          }
        : home.notesPage;
      notesError = '';
      if (!notesPage.notes.some((note) => note.id === selectedNoteId)) {
        selectedNoteId = notesPage.notes[0]?.id ?? '';
      }
      if (mode === 'empty' && selectedNoteId) {
        mode = 'view';
      }
      if (!peers.some((peer) => peer.id === activePeerId)) {
        openPeer(peers[0]);
      }
      if (options.message) statusMessage = options.message;
    } catch (error) {
      notesError = error instanceof Error ? error.message : String(error);
      statusMessage = `加载失败: ${notesError}`;
    } finally {
      notesLoading = false;
      notesLoadingMore = false;
    }
  }

  function navigate(action: PendingAction) {
    if (action.type === 'view') {
      selectedNoteId = action.note.id;
      mode = 'view';
      statusMessage = `已选择 ${shortNoteId(action.note.id)}`;
      return;
    }
    if (action.type === 'edit') {
      selectedNoteId = action.note.id;
      draftContent = action.note.content;
      draftTags = [...action.note.tags];
      mode = 'edit';
      statusMessage = `正在编辑 ${shortNoteId(action.note.id)}`;
      scheduleRecommendations();
      return;
    }
    if (action.type === 'create') {
      selectedNoteId = '';
      draftContent = '';
      draftTags = [];
      recommendedTags = [];
      mode = 'create';
      statusMessage = '正在创建新笔记';
      return;
    }
    mode = 'settings';
    statusMessage = '已打开设置';
  }

  function requestNavigation(action: PendingAction) {
    if (isSameTarget(action)) return;
    if (isDirty) {
      confirmState = { type: 'discard', action };
      return;
    }
    navigate(action);
  }

  function isSameTarget(action: PendingAction) {
    if (mode === 'settings' && action.type === 'settings') return true;
    if (mode === 'create' && action.type === 'create') return true;
    if ((mode === 'view' || mode === 'edit') && 'note' in action) {
      return action.note.id === selectedNoteId && ((mode === 'view' && action.type === 'view') || (mode === 'edit' && action.type === 'edit'));
    }
    return false;
  }

  function scheduleSearch() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void runSearch();
    }, 300);
  }

  async function runSearch() {
    clearTimeout(searchTimer);
    const normalized = searchInput.trim();
    activeQuery = normalized;
    statusMessage = normalized ? `已按“${normalized}”检索` : '已切回最近笔记流';
    await loadNotes();
  }

  function scheduleRecommendations() {
    clearTimeout(recommendTimer);
    const content = draftContent.trim();
    if (!content) {
      recommendedTags = [];
      return;
    }
    recommendTimer = setTimeout(async () => {
      try {
        recommendedTags = await getTagRecommendations({ content });
      } catch {
        recommendedTags = [];
      }
    }, 400);
  }

  function addTag(value: string) {
    const tag = value.trim();
    if (tag && !draftTags.includes(tag)) {
      draftTags = [...draftTags, tag];
    }
  }

  function removeTag(value: string) {
    draftTags = draftTags.filter((tag) => tag !== value);
  }

  async function saveNote() {
    const content = draftContent.trim();
    if (!content) {
      statusMessage = '请输入笔记内容';
      return;
    }
    isSaving = true;
    try {
      const wasEditing = mode === 'edit';
      const note =
        wasEditing && activeNote
          ? await editNoteRemote({ noteId: activeNote.id, content, tags: draftTags })
          : await createNoteRemote({ content, tags: draftTags });
      selectedNoteId = note.id;
      mode = 'view';
      recommendedTags = [];
      await loadNotes({ message: `${wasEditing ? '已生成新版本' : '已创建笔记'} ${shortNoteId(note.id)}` });
    } catch (error) {
      statusMessage = `保存失败: ${error instanceof Error ? error.message : String(error)}`;
    } finally {
      isSaving = false;
    }
  }

  function discardDraft() {
    if (mode === 'edit' && activeNote) {
      navigate({ type: 'view', note: activeNote });
      statusMessage = '已放弃当前修改';
    } else {
      mode = notes.length ? 'view' : 'empty';
      statusMessage = '已放弃新建内容';
    }
  }

  async function deleteNote(note: Note) {
    isDeleting = true;
    try {
      await deleteNoteRemote({ noteId: note.id });
      if (selectedNoteId === note.id) {
        selectedNoteId = '';
        mode = 'empty';
      }
      await loadNotes({ message: `已删除 ${shortNoteId(note.id)}` });
    } catch (error) {
      statusMessage = `删除失败: ${error instanceof Error ? error.message : String(error)}`;
    } finally {
      isDeleting = false;
      confirmState = undefined;
    }
  }

  function startResize(event: MouseEvent) {
    event.preventDefault();
    isResizing = true;
  }

  function resizeSidebar(event: MouseEvent) {
    if (!isResizing) return;
    sidebarWidth = clampSidebarWidth(window.innerWidth, event.clientX);
  }

  function stopResize() {
    if (!isResizing) return;
    isResizing = false;
    localStorage.setItem('synap_sidebar_width', String(sidebarWidth));
  }

  function openPeer(peer: Peer | undefined) {
    activePeerId = peer?.id ?? '';
    peerNoteDraft = peer?.note ?? '';
  }

  async function refreshSync(message = '同步状态已刷新') {
    try {
      const home = await getHome({ query: activeQuery });
      notesPage = home.notesPage;
      syncOverview = home.syncOverview;
      syncMessage = message;
    } catch (error) {
      syncMessage = `同步状态读取失败: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  async function runSyncAction(action: () => Promise<unknown>, message: string) {
    isSyncMutating = true;
    try {
      await action();
      await refreshSync(message);
    } catch (error) {
      syncMessage = error instanceof Error ? error.message : String(error);
    } finally {
      isSyncMutating = false;
    }
  }

  function statusLabel(status: string) {
    return status === 'Pending' ? '待信任' : status === 'Trusted' ? '已信任' : status === 'Retired' ? '已停用' : '已撤销';
  }

  function roleLabel(role: string) {
    return role === 'Initiator' ? '主动同步' : role === 'Listener' ? '监听接入' : role === 'RelayFetch' ? 'Relay 拉取' : 'Relay 推送';
  }

  function sessionStatusLabel(status: string) {
    return status === 'Completed' ? '完成' : status === 'PendingTrust' ? '待信任' : '失败';
  }
</script>

<svelte:head>
  <title>Synap</title>
  <meta name="description" content="Synap Web" />
</svelte:head>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  role="application"
  class="synap-app"
  on:mousemove={resizeSidebar}
  on:mouseup={stopResize}
  on:mouseleave={stopResize}
>
  <div class="sidebar" id="sidebar" style:width={`${sidebarWidth}px`}>
    <div class="sidebar-header">
      <span>Synap Web</span>
      <div class="header-actions">
        <button class="ui-icon-button ui-button--ghost" type="button" title="设置" on:click={() => requestNavigation({ type: 'settings' })}>⚙</button>
        <button class="ui-icon-button ui-button--ghost" type="button" title="刷新" on:click={() => loadNotes({ message: '已刷新笔记列表' })}>↻</button>
        <button class="ui-icon-button ui-button--ghost" type="button" title="新建笔记" on:click={() => requestNavigation({ type: 'create' })}>+</button>
      </div>
    </div>

    <div class="search-container">
      <div class="search-box">
        <input
          type="text"
          class="search-input"
          placeholder="搜索笔记或标签..."
          bind:value={searchInput}
          on:input={scheduleSearch}
          on:keydown={(event) => event.key === 'Enter' && runSearch()}
        />
        {#if searchInput.trim()}
          <button class="ui-icon-button ui-button--ghost search-clear-btn" type="button" title="清空搜索" on:click={() => { searchInput = ''; void runSearch(); }}>×</button>
        {/if}
        <button class="ui-icon-button ui-button--ghost search-btn" type="button" title="搜索" on:click={runSearch}>⌕</button>
      </div>
    </div>

    <div class="list-title">
      {#if notesLoading && notes.length === 0}
        正在加载笔记…
      {:else if activeQuery === ''}
        最近笔记 · {notes.length} 条
      {:else}
        搜索结果 · {notes.length} 条
      {/if}
    </div>

    <div class="sidebar-scroll-area">
      {#if notesLoading && notes.length === 0}
        <div class="list-empty">正在从 synap-core 载入数据...</div>
      {:else if notesError}
        <div class="list-empty error-text">加载失败: {notesError}</div>
      {:else if notes.length === 0}
        <div class="list-empty">{activeQuery === '' ? '还没有笔记，点击右上角 + 开始记录。' : '没有匹配结果，试试别的关键词。'}</div>
      {:else}
        <div class="note-list-container">
          <ul class="note-list" bind:this={listElement}>
            <div class="active-indicator" bind:this={indicatorElement}></div>
            {#each notes as note (note.id)}
              <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
              <li
                role="button"
                tabindex="0"
                data-note-id={note.id}
                class:active={selectedNoteId === note.id}
                class="note-item"
                on:click={() => requestNavigation({ type: 'view', note })}
                on:keydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    requestNavigation({ type: 'view', note });
                  }
                }}
              >
                <div class="note-item-actions">
                  <button class="ui-icon-button ui-button--ghost note-item-action" type="button" title="编辑笔记" disabled={isDeleting} on:click|stopPropagation={() => requestNavigation({ type: 'edit', note })}>✎</button>
                  <button class="ui-icon-button ui-button--ghost note-item-action note-item-action-danger" type="button" title="删除笔记" disabled={isDeleting} on:click|stopPropagation={() => (confirmState = { type: 'delete', note })}>⌫</button>
                </div>
                <div class="note-content">{preview(note.content)}</div>
                <div class="note-meta">
                  <span class="note-time">{formatTimestamp(note.created_at)}</span>
                  {#each note.tags as tag}
                    <span class="note-meta-tag">{tag}</span>
                  {/each}
                </div>
              </li>
            {/each}
          </ul>

          {#if activeQuery === '' && notesPage.next_cursor}
            <div class="note-list-footer">
              <button class="ui-button ui-button--ghost note-list-more" type="button" disabled={notesLoadingMore} on:click={() => loadNotes({ cursor: notesPage.next_cursor, append: true })}>
                {notesLoadingMore ? '正在加载更多…' : '加载更多'}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    role="separator"
    aria-orientation="vertical"
    tabindex="0"
    class:is-resizing={isResizing}
    class="resizer"
    on:mousedown={startResize}
  ></div>

  <div class="main-content">
    {#if mode === 'settings'}
      <div class="settings-container">
        <h2>设置</h2>

        <h3>同步与信任</h3>
        <div class="setting-card setting-card--stacked">
          <div class="setting-info">
            <h3>SSR 同步监听</h3>
            <p>Node/SvelteKit 迁移后当前页面直连 coreffi；TCP listener 尚未作为 coreffi 接口暴露，暂不在 Web 端自动启动。</p>
          </div>
          <div class="setting-actions">
            <button class="ui-button ui-button--primary" type="button" disabled={isSyncMutating} on:click={() => refreshSync('同步状态已刷新')}>
              {isSyncMutating ? '处理中…' : '刷新监听状态'}
            </button>
          </div>
        </div>

        {#if syncOverview}
          <div class="sync-settings-grid">
            <div class="setting-card setting-card--stacked sync-overview-card">
              <div class="sync-card-header">
                <div>
                  <div class="sync-eyebrow">监听状态</div>
                  <h3>未接管</h3>
                </div>
                <div class="sync-chip">Stopped</div>
              </div>
              <div class="sync-meta-grid">
                <div class="sync-meta-item"><span class="sync-meta-label">协议</span><strong>TCP</strong></div>
                <div class="sync-meta-item"><span class="sync-meta-label">后端</span><strong>Node/coreffi</strong></div>
                <div class="sync-meta-item"><span class="sync-meta-label">端口</span><strong>未分配</strong></div>
                <div class="sync-meta-item"><span class="sync-meta-label">局域网地址</span><strong>未检测到</strong></div>
              </div>
            </div>

            <div class="setting-card setting-card--stacked sync-overview-card">
              <div class="sync-card-header">
                <div>
                  <div class="sync-eyebrow">本机身份</div>
                  <h3>用于同步握手与设备识别</h3>
                </div>
              </div>
              <div class="identity-grid">
                <div class="identity-panel">
                  <div class="identity-panel-head">
                    <div class="sync-chip sync-chip--soft">{syncOverview.localIdentity.identity.kaomoji_fingerprint}</div>
                    <strong>通信身份</strong>
                  </div>
                  <code class="identity-public-key">{syncOverview.localIdentity.identity.display_public_key_base64}</code>
                </div>
                <div class="identity-panel">
                  <div class="identity-panel-head">
                    <div class="sync-chip sync-chip--soft">{syncOverview.localIdentity.signing.kaomoji_fingerprint}</div>
                    <strong>签名身份</strong>
                  </div>
                  <code class="identity-public-key">{syncOverview.localIdentity.signing.display_public_key_base64}</code>
                </div>
              </div>
            </div>
          </div>

          {#if syncMessage}
            <div class="settings-inline-message">{syncMessage}</div>
          {/if}

          <div class="settings-section-block">
            <div class="settings-section-heading">
              <div>
                <h3>设备信任</h3>
                <p>共 {peers.length} 台设备，待处理 {peers.filter((peer) => peer.status === 'Pending').length} 台。</p>
              </div>
              <button class="ui-button" type="button" on:click={() => refreshSync('设备列表已刷新')}>刷新列表</button>
            </div>

            <div class="sync-peer-list">
              {#if peers.length === 0}
                <div class="setting-card">
                  <div class="setting-info">
                    <h3>还没有发现设备</h3>
                    <p>当有设备尝试与当前 Web 节点同步后，这里会出现待处理或已信任的设备记录。</p>
                  </div>
                </div>
              {/if}

              {#each peers as peer}
                <div class="setting-card setting-card--stacked sync-peer-card">
                  <div class="sync-card-header">
                    <div>
                      <div class="sync-eyebrow">{statusLabel(peer.status)}</div>
                      <h3>{peer.note ?? '未命名设备'}</h3>
                    </div>
                    <div class="sync-chip">{peer.kaomoji_fingerprint}</div>
                  </div>
                  <div class="sync-peer-summary">
                    <span>{peer.algorithm} · {peer.display_public_key_base64}</span>
                  </div>
                  <div class="sync-peer-actions">
                    <button class="ui-button" type="button" on:click={() => openPeer(peer)}>管理</button>
                    {#if peer.status === 'Pending'}
                      <button class="ui-button ui-button--primary" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => approvePeerRemote({ peerId: peer.id, note: '' }), '设备已设为可信')}>直接信任</button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>

            {#if activePeer}
              <div class="setting-card setting-card--stacked sync-peer-detail-card">
                <div class="sync-card-header">
                  <div>
                    <div class="sync-eyebrow">设备详情</div>
                    <h3>{activePeer.note ?? '未命名设备'}</h3>
                  </div>
                  <button class="ui-button ui-button--ghost" type="button" on:click={() => openPeer(undefined)}>收起</button>
                </div>
                <div class="sync-peer-detail-grid">
                  <div class="sync-meta-item"><span class="sync-meta-label">状态</span><strong>{statusLabel(activePeer.status)}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">识别码</span><strong>{activePeer.kaomoji_fingerprint}</strong></div>
                </div>
                <label class="settings-field">
                  <span>备注</span>
                  <input class="settings-input" bind:value={peerNoteDraft} />
                </label>
                <div class="sync-peer-summary sync-peer-summary--full">
                  <span>{activePeer.display_public_key_base64}</span>
                </div>
                <div class="sync-peer-actions sync-peer-actions--detail">
                  <button class="ui-button" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => updatePeerNoteRemote({ peerId: activePeer.id, note: peerNoteDraft }), '设备备注已更新')}>保存备注</button>
                  <button class="ui-button ui-button--primary" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => approvePeerRemote({ peerId: activePeer.id, note: peerNoteDraft }), '设备已设为可信')}>设为可信</button>
                  <button class="ui-button" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => setPeerStatusRemote({ peerId: activePeer.id, status: 'Pending' }), '设备状态已更新为待信任')}>设为待处理</button>
                  <button class="ui-button" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => setPeerStatusRemote({ peerId: activePeer.id, status: 'Retired' }), '设备状态已更新为已停用')}>停用</button>
                  <button class="ui-button" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(() => setPeerStatusRemote({ peerId: activePeer.id, status: 'Revoked' }), '设备状态已更新为已撤销')}>撤销</button>
                  <button class="ui-button" type="button" disabled={isSyncMutating} on:click={() => runSyncAction(async () => { await deletePeerRemote({ peerId: activePeer.id }); openPeer(undefined); }, '设备记录已删除')}>删除记录</button>
                </div>
              </div>
            {/if}
          </div>

          <div class="settings-section-block">
            <div class="settings-section-heading">
              <div>
                <h3>最近同步</h3>
                <p>点击列表项查看本次同步的细节。</p>
              </div>
            </div>
            <div class="sync-session-list">
              {#if syncOverview.recentSyncSessions.length === 0}
                <div class="setting-card">
                  <div class="setting-info">
                    <h3>还没有同步记录</h3>
                    <p>当其他设备与当前 Web 节点完成握手并进入同步流程后，这里会出现统计信息。</p>
                  </div>
                </div>
              {/if}

              {#each syncOverview.recentSyncSessions as session}
                <button class="setting-card setting-card--stacked sync-session-card" type="button" on:click={() => (selectedSessionId = session.id)}>
                  <div class="sync-card-header">
                    <div>
                      <div class="sync-eyebrow">{roleLabel(session.role)}</div>
                      <h3>{session.peer_label ?? '未命名设备'}</h3>
                    </div>
                    <div class="sync-chip">{sessionStatusLabel(session.status)}</div>
                  </div>
                  <div class="sync-session-meta">
                    <span>{formatTimestamp(session.finished_at_ms)}</span>
                    <span>收 {session.records_received.toString()} / 发 {session.records_sent.toString()}</span>
                    <span>{session.duration_ms.toString()} ms</span>
                  </div>
                </button>
              {/each}
            </div>

            {#if selectedSession}
              <div class="setting-card setting-card--stacked sync-session-detail-card">
                <div class="sync-card-header">
                  <div>
                    <div class="sync-eyebrow">同步详情</div>
                    <h3>{selectedSession.peer_label ?? '未命名设备'}</h3>
                  </div>
                  <div class="sync-chip">{sessionStatusLabel(selectedSession.status)}</div>
                </div>
                <div class="sync-meta-grid">
                  <div class="sync-meta-item"><span class="sync-meta-label">开始时间</span><strong>{formatTimestamp(selectedSession.started_at_ms)}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">结束时间</span><strong>{formatTimestamp(selectedSession.finished_at_ms)}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">发送记录</span><strong>{selectedSession.records_sent.toString()}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">接收记录</span><strong>{selectedSession.records_received.toString()}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">应用记录</span><strong>{selectedSession.records_applied.toString()}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">跳过记录</span><strong>{selectedSession.records_skipped.toString()}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">发送字节</span><strong>{selectedSession.bytes_sent.toString()}</strong></div>
                  <div class="sync-meta-item"><span class="sync-meta-label">接收字节</span><strong>{selectedSession.bytes_received.toString()}</strong></div>
                </div>
                {#if selectedSession.error_message}
                  <div class="settings-inline-error">{selectedSession.error_message}</div>
                {/if}
              </div>
            {/if}
          </div>
        {:else}
          <div class="setting-card">正在读取同步状态…</div>
        {/if}

        <h3>备份和恢复</h3>
        <div class="setting-card">
          <div class="setting-info">
            <h3>数据备份</h3>
            <p>直接下载当前使用中的 redb 数据库文件。</p>
          </div>
          <form action="/api/settings/export" method="get">
            <button class="ui-button" type="submit">导出备份</button>
          </form>
        </div>

        <form class="setting-card setting-form" action="/api/settings/import" method="post" enctype="multipart/form-data">
          <div class="setting-info">
            <h3>恢复数据</h3>
            <p>上传 redb 数据库文件并替换当前数据库，页面会在导入后刷新。</p>
          </div>
          <div class="setting-actions">
            <input class="settings-file-input" type="file" name="database" accept=".redb,application/octet-stream" />
            <button class="ui-button ui-button--primary" type="submit">导入数据库</button>
          </div>
        </form>

        <h3 class="settings-about-title">关于</h3>
        <div class="settings-about-block">
          <h2>Synap</h2>
          <span class="subtitle-text">SvelteKit 重构版</span>
          <span class="main-text">一个基于有向无环图（DAG）的极简思维捕获与路由中枢。当前设置页提供本地 redb 数据库的直接导入与导出。</span>
        </div>
      </div>
    {:else if mode === 'empty'}
      <div class="empty-state">点击左上角 + 创建新笔记，或点击一条已有笔记开始查看</div>
    {:else if mode === 'view'}
      <div class="editor-container">
        <div class="editor-header">
          <h2>{editorTitle}</h2>
          <p class="editor-subtitle">{editorSubtitle}</p>
        </div>

        <div class="note-viewer">
          {#if activeNote}
            {#if activeNote.tags.length > 0}
              <div class="tag-bar tag-bar-readonly">
                {#each activeNote.tags as tag}
                  <span class="tag-pill tag-pill-readonly">{tag}</span>
                {/each}
              </div>
            {/if}
            <article class="note-viewer-body">{activeNote.content}</article>
          {:else}
            <div class="empty-state">选择一条笔记开始查看</div>
          {/if}
        </div>

        <div class="status-bar">
          <span>{statusMessage}</span>
          <span class="subtitle-text">{activeQuery === '' ? '数据源：最近笔记' : `当前检索：${activeQuery}`}</span>
        </div>
      </div>
    {:else}
      <div class="editor-container">
        <div class="editor-header editor-header-editing">
          <div>
            <h2>{editorTitle}</h2>
            <p class="editor-subtitle">{editorSubtitle}</p>
          </div>
          <div class="editor-actions">
            {#if isDirty}
              <span class="editor-dirty-indicator">未保存更改</span>
            {/if}
            <button class="ui-button ui-button--ghost editor-action editor-action-secondary" type="button" on:click={discardDraft}>放弃修改</button>
            <button class="ui-button ui-button--primary editor-action editor-action-primary" type="button" disabled={!canSave} on:click={saveNote}>
              {isSaving ? '正在保存...' : mode === 'edit' ? '保存为新版本' : '创建笔记'}
            </button>
          </div>
        </div>

        <div class="tag-bar">
          {#each draftTags as tag}
            <span class="tag-pill">
              {tag}
              <button class="ui-icon-button ui-button--ghost tag-delete" type="button" title="删除标签" on:click={() => removeTag(tag)}>×</button>
            </span>
          {/each}
          <input
            type="text"
            class="tag-input"
            placeholder="输入标签后回车"
            on:keydown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                addTag(event.currentTarget.value);
                event.currentTarget.value = '';
              }
            }}
          />
        </div>

        {#if availableRecommendedTags.length > 0}
          <div class="tag-suggestions">
            <span class="tag-suggestion-label">推荐标签</span>
            {#each availableRecommendedTags as tag}
              <button class="ui-button ui-button--default tag-suggestion" type="button" on:click={() => addTag(tag)}>{tag}</button>
            {/each}
          </div>
        {/if}

        <textarea id="editor" placeholder="点此开始记录笔记" bind:value={draftContent} on:input={scheduleRecommendations}></textarea>

        <div class="status-bar">
          <span>{statusMessage}</span>
          <span class="subtitle-text">{activeQuery === '' ? '数据源：最近笔记' : `当前检索：${activeQuery}`}</span>
        </div>
      </div>
    {/if}

    {#if confirmState}
      <div class="confirm-overlay" role="presentation">
        <div class="confirm-dialog" role="alertdialog" aria-modal="true">
          <div class="confirm-dialog-copy">
            {#if confirmState.type === 'delete'}
              <h3>删除笔记？</h3>
              <p>这条笔记会被标记为删除，之后不会再出现在最近笔记流里。</p>
            {:else}
              <h3>放弃未保存更改？</h3>
              <p>当前编辑内容尚未保存，继续操作会丢弃这些修改。</p>
            {/if}
          </div>
          <div class="confirm-dialog-actions">
            <button class="ui-button ui-button--ghost" type="button" on:click={() => (confirmState = undefined)}>
              取消
            </button>
            {#if confirmState.type === 'delete'}
              <button class="ui-button ui-button--danger" type="button" on:click={() => deleteNote((confirmState as { type: 'delete'; note: Note }).note)}>
                删除
              </button>
            {:else}
              <button class="ui-button ui-button--primary" type="button" on:click={() => { const action = (confirmState as { type: 'discard'; action: PendingAction }).action; confirmState = undefined; navigate(action); }}>
                放弃修改
              </button>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
