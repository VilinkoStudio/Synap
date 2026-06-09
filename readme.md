# Synap （积极开发中）

​一款极简的用于快速思维捕获的软件。

[简体中文 README](readme.md)（当前） | [English README](readme_en-us.md)

## 下载体验

目前软件仍在前期开发阶段，你可以前往项目的 [Github Release](https://github.com/VilinkoStudio/Synap/releases) 体验开发版本。

[软件官网](https://vilinkostudio.github.io/synap.vilinko.com/)

## 灵感来源

​传统笔记软件总是强迫你在记录前先思考：“这个想法该放进哪个文件夹？” ，这种强制分类带来的心理负担，往往会扼杀转瞬即逝的灵感。

所以，灵感的捕获与系统化整理，理应彻底分开。本软件不强迫你搭建宏大的知识库，只为你提供一个纯粹的界面，忠实记录意识的流转、发散与收束。它​像系统备忘录一样简洁、轻盈，但是拥有远超其之上的思想演化能力以及强大的检索与浏览能力，便于寻找与后续阅读。Synap 不解决"如何整理想法",它解决"无需整理也能找回想法"。

## 核心体验

* 快速捕获思维

  不同于传统笔记强制的“树形目录”与“分类标签”，回归记录本质。打开、写下、离开，没有任何多余动作，将认知带宽完全留给思考本身，想法的连接与结构化全部交给软件进行。

* 留存完整思考脉络和因果关系

  打破碎片化记录的壁垒，让你的想法彼此连接。你可以清晰地看到一个粗糙的念头，是如何一步步推演，最终得到一个结论的。它为你未来的复盘与回忆，保留了最完整的上下文信息。

* 完全本地存储数据，守护数据安全

  软件采用极其紧凑的单文件本地数据库。使用纯文本记录数据（兼容 Markdown），数千条笔记也仅需1MB，同时支持压缩与加密。

* 纯血原生开发，流畅丝滑

  我们厌倦了动辄数百兆、开启需要漫长等待的“套壳网页”应用。Synap 坚持纯血原生开发，动画丝滑流畅，响应时间极低。添加桌面小组件可以一键开始记录，毫不迟滞。

* 无缝汇总多端思想

  得益于软件底层独特的演化追踪架构，它无需担心多设备间数据冲突和数据丢失。无论在何时何地，当设备重新交汇时，各个设备的笔记都会无缝融合。

## 工程架构

本项目采用 Monorepo 组织结构。当前 Rust workspace 成员为 `core`、`corenet`、`coreffi-shared`、`coreffi`、`coreffi-uniffi029`、`cli`、`desktop_linux`、`relay`、`xtask`；`android` 与 `web` 分别由 Gradle 和 pnpm/Vite 管理。

* `core/`：Rust 逻辑内核。负责纯 Rust KV 数据落盘、不可变 DAG 状态机维护、读时过滤渲染算法以及同步协议。
* `corenet/`：跨设备同步网络层。负责发现、通道与同步服务的网络抽象。
* `coreffi-shared/`：UniFFI 共享 FFI facade。这里保存唯一维护的 `src/synap.udl`，以及 `error`、`types`、`service`、`adapter` 等 FFI-facing Rust 源码。
* `coreffi/`：UniFFI `0.31.0` adapter crate。面向 Android/Kotlin 与 Node/Web，导出 `uniffi_synap_coreffi`。
* `coreffi-uniffi029/`：UniFFI `0.29.4` adapter crate。面向 C# / WinUI，以及其他被 UniFFI 0.29 生态钉住的消费者，导出 `uniffi_synap_coreffi_uniffi029`。
* `cli/`：Rust 命令行前端。提供纯终端环境下的捕获、检索、图谱与同步入口。
* `desktop_linux/`：当前 Linux 桌面端实现。基于 Rust + GTK4 + libadwaita，仅面向 Linux 平台发布。
* `relay/`：零信任同步中继服务。
* `android/`：Kotlin 原生应用。Gradle 在构建期编译 `coreffi`，并接入自动生成的 UniFFI Kotlin 绑定。
* `xtask/`：Rust 工具目标。负责从 `coreffi-shared/src/synap.udl` 生成 Kotlin、C# 与 Node 绑定，并在 `target/xtask/uniffi-input/...` 下创建版本 adapter 专用的临时输入 crate。
* `web/`：SvelteKit + Vite 前端。通过生成的 Node UniFFI 绑定加载 `coreffi` 动态库。

UniFFI 结构约定：

* `coreffi-shared/src/synap.udl` 是唯一维护的 UDL。不要为不同平台复制 UDL。
* `coreffi-shared/src/*.rs` 是唯一维护的 FFI-facing Rust facade 源码。`coreffi` 与 `coreffi-uniffi029` 使用 `#[path = "../../coreffi-shared/src/..."]` 将这些模块纳入各自 adapter crate。这个做法是刻意保留的：UniFFI 生成的类型需要属于当前导出 crate，普通依赖 re-export 会破坏绑定元数据解析。
* 不同 UniFFI 版本只在 adapter crate 层分叉。平台代码只选择自己兼容的 adapter，不维护第二套 UDL 或第二套业务 FFI 层。
* `target/xtask/uniffi-input/...` 是生成工具输入，不是源码。它用于让 UniFFI/Node 生成器按 adapter crate 名解析元数据，不能提交进版本管理。

当前平台策略：

* Linux：已提供 `desktop_linux`，作为当前唯一维护中的桌面端实现。
* Windows：原生桌面端采用 C# + WinUI + UniFFI 共享 Rust 核心。由于 `uniffi-bindgen-cs v0.10.0` 对应 UniFFI `0.29.4`，Windows 当前使用 `coreffi-uniffi029`，但 UDL 与 FFI facade 仍来自 `coreffi-shared`。
* 苹果生态：当前不计划兼容 macOS、iOS 或其他 Apple 平台，也没有对应的发布计划。

## 构建与运行

确保已安装 Rust 工具链。若需要构建 Android，还需要可用的 JDK、Android SDK 与 NDK。

进入工作区根目录，编译命令行工具：

```bash
cargo build --release -p synap-cli
./target/release/synap --help
```

运行桌面端：

```bash
cargo run -p synap-desktop-linux
```

构建 Android 调试包：

```bash
cd android
./gradlew assembleDebug
```

Android 的 `preBuild` 会先执行两件事：

* 编译 `coreffi` 对应的 Android 动态库 `libuniffi_synap_coreffi.so`。
* 通过 `cargo run -p xtask -- gen-uniffi-kotlin --udl coreffi-shared/src/synap.udl --config coreffi/uniffi.toml ...` 生成 Kotlin UniFFI 绑定到 `android/app/build/generated/...`。

生成 Web/Node 绑定：

```bash
pnpm --dir web prepare:bindings
```

该命令会调用 `xtask gen-uniffi-node`，构建 `synap-coreffi` cdylib，生成 `target/generated/nodejs/synap-coreffi`，并复制当前平台动态库。Web 服务端通过生成的 ESM 包手动 `load()` 这个动态库。

启动 Web 应用：

```bash
pnpm --dir web dev
```

检查 Web 类型：

```bash
pnpm --dir web check
```

生成 C# 绑定时使用 0.29 adapter：

```bash
cargo run -p xtask -- gen-uniffi-csharp \
  --udl coreffi-shared/src/synap.udl \
  --config coreffi-uniffi029/uniffi.toml \
  --out-dir desktop_windows/obj/Generated/UniFFI \
  --crate-name uniffi_synap_coreffi_uniffi029
```

## 仓库约定

这个仓库是单仓多端，但不要求每个 feature 一次性完成全平台适配。主线维护的是“共享核心 + 当前正在维护的平台集合”，不是“所有壳层永远同步完成”。约定如下：

* 不直接在 `master` 上做长期开发。新功能、新重构、新实验统一从 `feat/*`、`refactor/*`、`spike/*` 分支开始，`master` 只保留可运行、可回退的提交。
* 一个跨端功能按“层”拆提交，而不是按“今天改了什么”拆提交。推荐顺序是 `core/` -> `coreffi-shared/` -> `coreffi|coreffi-uniffi029/` -> `android|desktop|cli|web/` -> `docs|build`。
* 一个 feature 可以逐个平台落地，但合入 `master` 的提交不应该把本次涉及的平台直接打坏。
* 允许本地存在 WIP 提交，但整理进主线前需要压成一串可解释、可回退的提交。
* 构建产物、本地数据库、UniFFI 生成绑定、`jniLibs` 与其他本地产物不进入版本管理；它们应由构建流程重新生成。
* 长周期并行开发优先使用 `git worktree`，不要把所有事情都堆在一个工作区里。

一个典型的跨端功能建议这样推进：

```bash
git switch -c feat/note-tag-flow
git add core coreffi
git commit -m "feat(core): add note tag service flow"
git add android/app/src/main/java android/app/src/main/AndroidManifest.xml android/app/build.gradle.kts
git commit -m "feat(android): wire note tag flow into app"
```

如果某个功能会做很多天，额外开一个工作树会更稳：

```bash
git worktree add ../synap-note-tag feat/note-tag-flow
```
