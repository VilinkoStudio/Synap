# Synap

一个基于有向无环图（DAG）的极简思维捕获与路由中枢。

本项目旨在消除传统笔记软件中由于“强制分类”和“树形目录”带来的心智负担。它不要求你在记录前进行结构化思考，而是忠实地记录意识的流转、发散与收束。

## 核心理念

* 零阻力捕获：放弃文件夹概念，即开即写，将灵感捕获与系统化整理彻底解耦。
* 绝对不可变账本：放弃传统关系型数据库，拥抱纯 Rust KV 数据库（Key-Value）。底层仅存在“文本块”与“语义指针”，所有操作严格遵循只读追加（Append-Only），绝不执行原地的修改与删除。
* 读时过滤视图：冲突解决不再发生于写入或同步阶段。系统通过“滤网（Reducer）”在读取时动态跨越死神指针和重定向指针，将碎片化的历史事实渲染为连贯的网状图谱。
* 极简去中心化同步：由于底层数据的不可变性与全局唯一 ID，多端同步被彻底降维为毫无心智负担的“集合求并集（Set Union）”，从物理法则上断绝并发冲突。
* 类 ZFS 滞后回收：借鉴写时复制（CoW）哲学，思维的废弃与修改仅产生新的指针。真正的物理存储释放，交由本地异步的标记-清除（Mark and Sweep）垃圾回收机制在离线时静默完成。

## 工程架构

本项目采用 Monorepo 组织结构。核心业务逻辑由 Rust 统一实现，以此驱动各个平台的原生外壳：

* `core/`：Rust 逻辑内核。负责纯 Rust KV 数据落盘、不可变 DAG 状态机维护、读时过滤渲染算法以及后台 GC 机制。
* `cli/`：Rust 命令行前端。提供纯终端环境下极速的捕获与回溯体验。
* `desktop/`：Rust 桌面端 UI。
* `android/`：Kotlin 原生应用。通过底层绑定（FFI）直接调用 `core` 编译的动态链接库。
* `web/`：Vue 3 前端应用。作为网络节点的轻量级可视化视图。

## 构建与运行

确保已安装 Rust 工具链。

进入工作区根目录，编译命令行工具：

```bash
cargo build --release -p cli
./target/release/cli --help
```

## Git 工作约定

这个仓库是单仓多端，`core`、`coreffi`、`android`、`desktop`、`web` 经常会围绕同一个功能一起变化。为了避免一个功能做很久后工作区失控，约定如下：

* 不直接在 `master` 上做长期开发。新功能、新重构、新实验统一从 `feat/*`、`refactor/*`、`spike/*` 分支开始，`master` 只保留可运行、可回退的提交。
* 一个跨端功能按“层”拆提交，而不是按“今天改了什么”拆提交。推荐顺序是 `core/` -> `coreffi/` -> `android|desktop|web|cli/` -> `docs|build`。
* 允许本地存在 WIP 提交，但每个提交都只解决一个明确问题，并尽量保证至少能编译，或者能清楚说明为什么暂时不能编译。
* 构建产物、本地数据库、生成绑定代码不进入版本管理；它们应该由构建流程重新生成。
* 长周期并行开发优先使用 `git worktree`，不要把所有事情都堆在一个工作区里，也尽量少依赖长期 `stash`。

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
