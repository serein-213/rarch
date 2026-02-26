# rarch 贡献指南

首先，感谢您考虑为 **rarch** 做出贡献！正是像您这样的人，让开源社区成为了一个如此美妙的地方。

## 行为准则
通过参与本项目，您同意遵守我们的 [行为准则](CODE_OF_CONDUCT_ZH.md)。

## 快速入门

### 前提条件
- **Rust 工具链**：您需要安装最新的稳定版 Rust。请通过 [rustup](https://rustup.rs/) 安装。
- **Git**：确保您的机器上已安装并配置好 Git。

### 开发工作流
1. **Fork 本仓库**：在 GitHub 上创建您个人的 Fork。
2. **克隆 Fork**：
   ```bash
   git clone https://github.com/Serein-213/rarch.git
   cd rarch
   ```
3. **创建分支**：使用具有描述性的名称（例如：`feat/simd-hashing` 或 `fix/journal-overflow`）。
4. **进行修改**：确保您的代码符合现有的风格和规范。
5. **运行测试**：
   ```bash
   cargo test
   ```
6. **代码规范检查**：我们使用 `clippy` 和 `rustfmt` 来维持代码质量。
   ```bash
   cargo fmt --all -- --check
   cargo clippy -- -D warnings
   ```

## 合并请求 (Pull Request) 流程
- 请确保每个 PR 仅针对单个问题或功能。
- 如果您的更改修改了用户界面或配置架构，请同步修改文档。
- 所有 PR 在合并前至少需要一名维护者的批准，并且必须通过所有 CI 检查。

## 报告问题
在提交 Issue 时，请使用提供的 [Bug 反馈](.github/ISSUE_TEMPLATE/bug_report_zh.md) 或 [功能请求](.github/ISSUE_TEMPLATE/feature_request_zh.md) 模板。
