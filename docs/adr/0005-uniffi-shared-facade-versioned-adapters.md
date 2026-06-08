# ADR-0005：共享 UniFFI facade 与版本化 adapter crate

## 状态

已接受

## 背景

Synap 需要把同一个 Rust 核心暴露给多个平台：

- Android/Kotlin 当前可以使用 UniFFI `0.31.0`
- Node/Web 当前走 `uniffi-bindgen-node-js`，也跟随当前 `coreffi` adapter
- C# / WinUI 受 `uniffi-bindgen-cs v0.10.0` 限制，需要 UniFFI `0.29.4`

如果直接把整个 `coreffi` 降到 `0.29.4`，会牺牲 Android 与 Node/Web 当前可用的新版本链路。如果为 C#、Android、Node/Web 各自维护一份 UDL 和 FFI facade，又会产生重复接口定义、重复类型转换和长期漂移风险。

## 决策驱动因素

- UDL 必须只有一份维护来源
- FFI-facing Rust facade 也必须只有一份维护来源
- 不同 UniFFI 版本可以共存，但只能在 adapter crate 层分叉
- 生成绑定的临时输入可以存在于 `target/`，不能成为需要维护的源码副本
- 平台代码应该选择兼容的 adapter，而不是拥有自己的 FFI 层

## 备选方案

### 方案 A：全部降到 UniFFI 0.29.4

优点：

- C# 兼容最直接
- adapter crate 数量最少

缺点：

- Android 与 Node/Web 被迫跟随较旧 UniFFI
- Node 绑定链路可能失去当前可用的 `0.31.0` 生态能力
- 后续升级会再次牵动所有平台

### 方案 B：每个平台维护自己的 UDL 和 FFI facade

优点：

- 每个平台可以自由选择 UniFFI 版本
- 短期接入简单

缺点：

- UDL、DTO、错误类型和 service facade 会重复
- 接口漂移很难审查
- 平台差异会反向污染核心 API 设计

### 方案 C：共享 UDL/FFI facade，按 UniFFI 版本维护 adapter crate

优点：

- UDL 和 FFI facade 只有一份源码
- C# 可以使用 `0.29.4`，Android/Node/Web 可以继续使用 `0.31.0`
- 版本差异被限制在 adapter crate 与生成工具边界
- 后续如果 C# 生态升级，只需要调整或移除 `coreffi-uniffi029`

缺点：

- workspace 中会同时存在两个 UniFFI 版本
- adapter crate 需要用 `#[path = "..."]` 纳入共享源码，而不是普通依赖 re-export
- `xtask` 需要为生成器准备 adapter-local 临时输入 crate

## 最终决策

采用 **方案 C：共享 UDL/FFI facade，按 UniFFI 版本维护 adapter crate**。

落地结构为：

- `coreffi-shared/`
  - 保存唯一维护的 `src/synap.udl`
  - 保存唯一维护的 FFI-facing Rust facade：`error.rs`、`types.rs`、`service.rs`、`adapter.rs`
- `coreffi/`
  - UniFFI `0.31.0`
  - 导出 `uniffi_synap_coreffi`
  - 面向 Android/Kotlin 与 Node/Web
- `coreffi-uniffi029/`
  - UniFFI `0.29.4`
  - 导出 `uniffi_synap_coreffi_uniffi029`
  - 面向 C# / WinUI
- `xtask/`
  - 统一生成 Kotlin、C#、Node 绑定
  - 在 `target/xtask/uniffi-input/...` 生成 adapter-local 临时输入 crate

`coreffi` 与 `coreffi-uniffi029` 使用 `#[path = "../../coreffi-shared/src/..."]` 引入共享源码。这不是临时技巧，而是当前 UniFFI 绑定元数据的约束：UDL 中定义的类型与函数需要属于实际导出 scaffolding 的 adapter crate。普通依赖 re-export 会让生成器在 crate metadata 和导出符号上解析到错误边界。

## 后果

### 正面影响

- 不维护多份 UDL
- 不维护多份 FFI facade
- C# 兼容 UniFFI `0.29.4`，同时不阻塞 Android/Node/Web 使用 UniFFI `0.31.0`
- 生成命令集中在 `xtask`，平台脚本只负责调用正式工具入口
- 后续平台接入只需要选择 adapter，不需要复制接口层

### 负面影响

- workspace 同时解析两个 UniFFI 版本，依赖图更复杂
- adapter crate 的源码结构看起来不如普通模块依赖直观
- 生成器临时输入 crate 需要额外文档说明，避免被误认为维护源码

### 当前约束

- 任何 FFI API 变更先改 `coreffi-shared/src/synap.udl` 和共享 facade 源码
- 不在 `coreffi/` 或 `coreffi-uniffi029/` 下新增平台专属 DTO 或 service API
- 不提交 `target/xtask/uniffi-input/...` 或 `target/generated/...`
- 如果未来 C# 生成器支持 UniFFI `0.31.0` 或更高版本，应新增 ADR 或更新实现计划，再评估是否移除 `coreffi-uniffi029`
