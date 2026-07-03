# Desktop Linux GTK Design

## Technology Choices

- UI toolkit: `gtk4` + `libadwaita` via `gtk-rs`
- Framework: `relm4` (reactive Elm-like architecture)
- Rust crate: `gtk4 = 0.9.6`, `libadwaita = 0.7.2`, `relm4 = 0.9.1`
- Core integration: `DesktopCore` trait → `SynapCoreAdapter` → `SynapService`

## Module Layout

```text
desktop_linux/src/
  main.rs              # 启动入口，初始化 core adapter 与 GTK Application
  app.rs               # App struct, view! macro, init, update, UI sync
  app/
    message.rs         # AppMsg enum — all messages
    browse.rs          # 浏览模式：导航、列表、搜索、分页
    focus.rs           # 沉浸模式：阅读、编辑、回复、删除/恢复
    sync_ops.rs        # 同步操作：连接、配对、信任
  core.rs              # DesktopCore trait + SynapCoreAdapter
  domain.rs            # AppState、HomeData、ContentView、FocusMode、视图模型
  usecase.rs           # load_home、load_note_detail 等聚合读取
  ui/
    mod.rs
    shell.rs           # 页面构建：笔记列表、阅读、标签、时间线、设置
    note_widgets.rs    # 笔记卡片、标签芯片
    theme.rs           # 主题切换、CSS 加载
    style.css          # 全局样式（扁平风）
    util.rs            # 纯函数工具：parse_tags、render_reading_text
    editor/
      mod.rs
      widget.rs        # WysiwygEditor 主组件
      model.rs         # MdBlock、BlockKind 文档模型
      parser.rs        # pulldown-cmark → Vec<MdBlock>
      renderer.rs      # MdBlock → GTK widget
      markdown.rs      # inline markdown → Pango markup
      editor.css       # 编辑器样式
```

## Data Flow

```text
GTK Widgets (view! macro)
  → AppMsg (message passing)
  → App::update() (message handler)
  → usecase::load_home / load_note_detail (aggregation)
  → DesktopCore trait (adapter)
  → SynapService (core)
  → redb + indexes (storage)
```

## UI Structure

主界面使用 `OverlaySplitView` 实现两态切换：

### Browse 模式（侧边栏展开）
- 左侧 Sidebar：导航列表（笔记、标签、时间线、回收站、设置）
- 右侧内容区：笔记卡片列表 + 搜索框 + 标签筛选

### Focus 模式（侧边栏折叠）
- 全屏阅读视图（WYSIWYG markdown 渲染）
- 右侧上下文面板（溯源链、回复、版本演化）
- 编辑模式（点击块 → 编辑，blur → 重新渲染）

## Features

- ✅ 笔记 CRUD（新建、编辑生成新版本、回复、删除、恢复）
- ✅ 搜索（200ms 防抖）
- ✅ 标签管理（标签列表、按标签筛选笔记）
- ✅ 时间线（会话浏览）
- ✅ 回收站（恢复、本地搜索过滤）
- ✅ 设置（主题切换、同步配置）
- ✅ 同步（监听、发现、配对、信任、连接管理）
- ✅ 键盘快捷键（Ctrl+N、Ctrl+F、Escape、Ctrl+Delete）
- ✅ 删除确认对话框
- ✅ Toast 通知

## Style Direction

扁平 + 圆角 + 大面积留白 + 组件紧凑。无阴影、无光晕、无装饰。
空间留给内容呼吸，不留给 chrome。
