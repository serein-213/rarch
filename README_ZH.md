# 归藏 (rarch)

[English](README.md) | [简体中文](README_ZH.md)

> **The Robust File Organizer** — 基于 Rust 构建的高性能、内容感知且具备原子化撤销能力的文件治理专家。

[![构建状态](https://img.shields.io/badge/status-active-brightgreen.svg)]()
[![开发语言](https://img.shields.io/badge/language-Rust-orange.svg)]()
[![开源协议](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)]()

## 为什么选择归藏？

市面上大多数文件整理工具仅简单依赖文件后缀名。**归藏 (rarch)** 专为追求数据完整性、存储效率和极致操作体验的高级用户设计。

### 视觉演示

![rarch UI](assets/rarch_tui.png)
*rarch 交互式 TUI 仪表盘*

![rarch CLI](assets/rarch_cli.png)
*rarch CLI 运行预览（Dry-Run）*

### 核心特性

- **极速引擎**: 由 Rust 和 `rayon` 驱动的并行处理逻辑。秒级完成 10 万级文件的扫描与归档。
- **原子化撤销 (Undo)**: 每一笔移动都有交易级日志记录。如果发现规则写错，`rarch undo` 能将所有文件精准还原至原位。
- **内容感知**: 拒绝后缀名欺骗。归藏利用深度二进制头（Magic Number）识别，即使 `.png` 被重命名为 `.txt` 也能准确归位。
- **智能语义去重**: 自动检测内容完全一致的文件（基于 SHA-256），并通过创建**硬链接**（Hard Link）消除冗余，在不改变目录结构的前提下瞬间拯救硬盘空间。
- **实时守望模式**: 运行 `rarch watch` 后，下载文件夹将从此告别混乱。文件在进入目录的瞬间即被自动组织。
- **交互式仪表盘 (TUI)**: 为键盘发烧友设计的终端美学界面。

## 安装

```bash
cargo install rarch --features ui
```

## 使用指南

### 1. 配置规则

创建 `rarch.toml`:

```toml
[[rules]]
name = "照片"
extensions = ["jpg", "png", "heic"]
target = "Pictures/Sorted"

[[rules]]
name = "文档"
extensions = ["pdf", "docx"]
target = "Documents/Work"
```

### 2. 批量整理

```bash
# 首先进行干跑预览（不实际移动文件）
rarch run --dry-run

# 执行正式整理与去重
rarch run --path ~/Downloads
```

### 3. 撤销操作

如果您对结果不满意：

```bash
rarch undo
```

### 4. 实时监控

开启后即可静默后台自动整理：

```bash
rarch watch --path ~/Downloads
```

## 架构设计

1. **扫描层 (Scanner)**: 支持深度或浅层目录遍历。
2. **核心引擎 (Engine)**:
    - 并行哈希计算 (SHA-256)。
    - 文件内容特征推断。
    - 基于链接的去重分支逻辑。
3. **日志层 (Journal)**: 基于 JSON 的事务日志，确保 Undo 操作 100% 可靠。
4. **交互层 (UI)**: 基于 `ratatui` 构建的零依赖终端用户界面。

## 许可证

MIT OR Apache-2.0
