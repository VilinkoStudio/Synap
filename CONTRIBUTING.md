# 贡献指南

本仓库是一个 monorepo，包含多个平台端与共享模块。为了让提交历史更清晰、发布流程更稳定，请统一遵循下面的提交与版本标签约定。

## 提交规范

本仓库遵循 Conventional Commits，推荐格式如下：

```text
<type>(<scope>): <summary>
```

示例：

```text
feat(android/net): 优化网络性能
fix(web/auth): 修复登录态过期后的跳转问题
refactor(core/search): 简化查询流水线
chore(xtask): 改进发布脚本输出
```

常用 `type` 含义：

- `feat`：新功能
- `fix`：缺陷修复
- `refactor`：不改变行为的重构
- `perf`：性能优化
- `docs`：仅文档变更
- `test`：仅测试相关变更
- `build`：构建系统、依赖或打包相关变更
- `ci`：CI/CD 工作流相关变更
- `chore`：其他维护性变更

## Monorepo Scope 约定

由于本仓库是 monorepo，`scope` 需要尽量明确说明“主要改动发生在哪个模块或平台”。

推荐优先使用以下形式：

```text
<platform-or-module>/<area>
```

例如：

- `android/net`
- `android/ui`
- `cli/config`
- `cli/output`
- `web/auth`
- `web/player`
- `desktop/window`
- `core/search`
- `core/audio`
- `coreffi/bindings`
- `xtask/release`

如果改动较小，且只影响某个顶层模块，也可以只写一级 scope：

```text
feat(android): 增加离线缓存
fix(cli): 修正帮助信息
docs(web): 补充部署环境变量说明
```

## 如何选择 Scope

- 如果改动只影响某一个平台，优先使用该平台作为主 scope。
- 如果改动集中在平台下的某个子领域，使用 `平台/领域` 的形式。
- 如果改动发生在共享 Rust 逻辑中，优先使用 `core/...` 或 `coreffi/...`。
- 如果改动主要影响构建、脚本或自动化流程，优先使用 `xtask`、`ci` 或 `build`。
- 如果一个提交同时涉及多个平台，优先选择“主要改动所在模块”作为 scope；如果没有明显主模块，可以使用更宽泛的 scope，例如 `core`、`build` 或 `repo`。

示例：

- 纯 Android 改动：`feat(android/net): 优化网络性能`
- 纯 CLI 改动：`fix(cli/config): 支持从自定义路径读取配置`
- 纯 Web 改动：`feat(web/editor): 支持拖拽上传`
- 多端共用核心逻辑改动：`refactor(core/search): 改进分词处理流程`
- GitHub Actions 工作流改动：`ci(github): 按平台限制发布标签`

## Summary 写法建议

`summary` 建议简短、明确，使用祈使句或描述当前变更目的，避免模糊表达。

推荐：

- `feat(android/net): 优化网络性能`
- `fix(web/auth): 重试前刷新 token`

避免：

- `feat(android/net): 优化了很多网络相关内容`
- `fix(cli): 修复一些问题`

## 版本 Tag 约定

各平台的发布不使用统一的全局 tag，而是使用“平台前缀 + 版本号”的方式触发对应 release 流程。

当前约定如下：

- Android：`android-vX.Y.Z`
- CLI：`cli-vX.Y.Z`
- Web：`web-vX.Y.Z`

示例：

```text
android-v1.0.0
cli-v1.2.3
web-v0.9.0
```

如果是预发布版本，也请直接体现在版本号中。推荐遵循语义化版本的预发布标记写法，例如：

```text
android-v1.0.0-alpha.1
android-v1.0.0-beta.1
android-v1.0.0-rc.1
cli-v2.3.0-beta.2
web-v0.9.0-rc.1
```

版本阶段约定如下：

- `alpha`：用于内部测试，功能可能还在快速变化，不保证稳定性
- `beta`：用于公开测试，功能基本完整，但仍可能存在已知问题或兼容性问题
- `rc`：Release Candidate，预选发布版本；如果未发现阻塞性问题，应直接转为正式 release
- 正式版：不带预发布后缀，例如 `android-v1.0.0`

关于 `rc` 的约定：

- `rc` 应尽量与最终正式版保持一致
- 如果 `rc` 测试期间没有发现需要修复的问题，可以直接以相同代码内容发布正式版
- 如果 `rc` 阶段发现问题，应修复后重新发布新的 `rc` 版本，例如 `rc.2`

对应关系如下：

- `android-v1.0.0` 触发 Android 发布流程
- `cli-v1.0.0` 触发 CLI 发布流程
- `web-v1.0.0` 触发 Web 发布流程

普通提交推送到 `master` 时，仍然会触发常规构建流程，不受上述发布 tag 规则影响。

## 建议工作流

在提交 PR 或创建发布前，建议检查以下事项：

1. 提交信息是否符合 Conventional Commits。
2. `scope` 是否准确表达了本次主要改动的平台或模块。
3. 如果要发布某个平台，是否创建了对应格式的 tag。
4. 尽量避免把多个不相关平台的改动混在同一个提交中。
5. 如果是内部测试、公开测试或候选发布，请使用 `alpha`、`beta`、`rc` 预发布后缀。

## 快速示例

```text
feat(android/net): 优化网络性能
fix(android/player): 避免重复播放
feat(cli/export): 增加 json 输出
fix(web/auth): 修复登录态过期后的跳转问题
refactor(core/index): 减少不必要的内存分配
docs(repo): 补充提交与发布约定
ci(github): 按平台限制发布标签
```
