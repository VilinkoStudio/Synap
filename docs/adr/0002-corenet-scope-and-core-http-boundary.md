# ADR-0002：明确 corenet 范围与 core 中普通 HTTP 能力边界

## 状态

已接受

## 背景

ADR-0001 决定将同步系统中平台差异明显的网络与发现能力外置，避免 `core` 直接承担 mDNS、监听、连接生命周期、Android 权限与前后台策略等复杂运行时。

随着 Synap 继续扩展，`core` 除了本地数据、DAG 状态机、查询渲染和同步协议外，还会承载一些跨端一致的业务能力。例如：

- 通过 HTTP 访问 relay 服务，实现跨局域网的数据流转
- 通过 HTTP 访问 embedding API，支持语义理解、语义索引和语义检索

这些能力也属于网络访问，但它们与 ADR-0001 讨论的局域网同步 transport 并不相同。局域网同步 transport 涉及发现、监听、入站连接和平台生命周期；普通 HTTP API 调用通常是主动式、短生命周期、请求响应式的业务访问。

如果把所有网络相关代码都外置，`core` 的共享业务能力会被切碎，宿主层需要重复实现 relay 和 embedding 访问逻辑。反过来，如果把所有网络运行时都塞回 `core`，又会破坏 ADR-0001 想保护的边界。

因此需要单独明确 `corenet` 的职责范围，以及 `core` 中普通 HTTP 能力的组织方式。

## 决策驱动因素

- `core` 是 Synap 的共享业务内核，不是纯协议层
- `core` 需要保持多端复用，但不应承担平台生命周期运行时
- `corenet` 的主要价值是隔离 mDNS、监听、连接管理等平台相关同步 transport
- relay HTTP、embedding HTTP 这类主动式请求在各端的架构差异较小
- 业务协议与 HTTP 细节需要分层，否则纯协议和业务状态机难以测试
- Android 侧应继续避免被 Rust 侧监听、发现、后台保活逻辑牵制

## 备选方案

### 方案 A：所有网络访问都放在 core 外

做法：

- `core` 只暴露纯业务协议和数据操作
- relay HTTP、embedding HTTP 都由宿主层或外部 crate 实现
- `core` 通过接口接收外部结果

优点：

- `core` 的依赖最少
- 纯逻辑测试最直接
- 平台可以完全选择自己的 HTTP 客户端

缺点：

- 宿主层重复实现跨端一致的业务访问逻辑
- relay 和 embedding 的错误语义容易分散
- `coreffi` 需要暴露更多细碎接口
- 对桌面端、CLI、Web Rust 宿主不够开箱即用

### 方案 B：所有网络访问都放入 corenet

做法：

- `corenet` 统一承载 TCP / mDNS / relay HTTP / embedding HTTP 等网络能力
- `core` 只依赖 `corenet` 的结果或通道

优点：

- 网络代码名义上集中
- `core` 可以不直接依赖 HTTP 客户端

缺点：

- `corenet` 会从“局域网同步 transport”膨胀成泛网络工具箱
- relay、embedding 这类业务能力与局域网发现、监听混在一起
- 模块命名和职责不清晰
- 后续容易把平台运行时重新绕回共享业务路径

### 方案 C：corenet 只负责平台相关同步 transport，普通 HTTP 业务能力进入 core 并在 core 内部分层

做法：

- `corenet` 只负责局域网同步所需的 TCP / mDNS / 监听 / 连接生命周期等能力
- relay HTTP、embedding HTTP 等主动式业务 HTTP 能力可以放入 `core`
- `core` 内部必须区分纯协议 / 业务状态机 / HTTP 适配层
- 宿主层继续负责何时调用、后台调度、网络条件、权限和用户配置

优点：

- 符合 `core` 作为共享业务内核的定位
- 避免宿主重复实现跨端一致的 HTTP 业务逻辑
- 保留 ADR-0001 对平台相关运行时的隔离
- `corenet` 职责清晰，不变成泛网络模块
- 纯协议和业务状态机仍可脱离 HTTP 进行测试

缺点：

- `core` 会引入 HTTP 客户端相关依赖
- `core` 的内部边界需要更严格维护
- 若未来支持 wasm/browser 或特殊平台，HTTP 实现可能需要 feature 或替换策略

## 最终决策

采用 **方案 C**。

明确以下原则：

1. `corenet` 的职责限定为局域网同步 transport，主要包括 TCP 连接、监听、mDNS / NSD 类发现、连接生命周期和相关错误边界。
2. `corenet` 不承载 relay HTTP、embedding HTTP 或其他普通业务 API 调用。
3. `core` 可以承载确定的、主动式、短生命周期 HTTP 业务能力，例如 relay client 和 embedding provider。
4. `core` 内部必须把纯协议、业务状态机和 HTTP 适配层分开组织，避免业务逻辑直接散落拼接 HTTP 请求。
5. 平台生命周期相关行为仍不进入 `core`，包括后台定时任务、网络状态监听、自动重试调度、Android WorkManager / Foreground Service 等。

## 决策说明

本决策细化 ADR-0001，而不是推翻 ADR-0001。

ADR-0001 关注的是同步系统的运行时边界，尤其是 mDNS、监听、入站连接、连接保活、系统权限和 Android 生命周期。这些能力平台相关性强，应该由 `corenet` 或宿主层处理。

普通 HTTP API 调用的性质不同。relay HTTP 和 embedding HTTP 是主动发起的请求响应式业务能力，它们的跨平台差异主要体现在配置、权限、证书、代理、超时等参数，而不是架构模型本身。因此它们可以进入 `core`，作为共享业务内核的一部分。

但是，进入 `core` 不意味着网络细节可以污染纯协议。推荐的内部结构是：

- `model`：请求、响应、密文封包、语义向量等数据模型
- `protocol`：不依赖真实网络的协议和状态机
- `service`：面向 `SynapService` 或业务门面的编排逻辑
- `http` / `provider`：具体 HTTP 请求实现

纯协议和业务状态机应能通过内存假实现或固定输入输出测试，不需要真实 HTTP 服务。

## 后果

### 正面影响

- `core` 的共享业务能力更完整
- relay 和 embedding 的跨端行为更容易保持一致
- `corenet` 边界清晰，只服务局域网同步 transport
- Android 仍然不会被 mDNS、监听、连接生命周期等 Rust 运行时绑住
- 纯协议逻辑可以在 `core` 内独立测试

### 负面影响

- `core` 会承担更多依赖管理压力
- HTTP 相关 feature、错误类型和配置需要谨慎设计
- 若 HTTP 代码与业务协议混写，测试和维护成本会快速上升

### 需要遵守的落地规则

- 不要把 mDNS、监听、入站连接和后台保活放入 `core`
- 不要把 relay HTTP 或 embedding HTTP 放入 `corenet`
- `core` 中的 HTTP 代码必须位于明确的适配层
- 纯协议和业务状态机不能依赖真实网络才能测试
- 宿主层负责触发时机、后台调度、权限、用户配置和平台网络策略

## 相关决策

- ADR-0001：core 保持平台无关，网络与发现能力外置
- ADR-0003：引入零信任 relay 作为去中心化同步的补充
