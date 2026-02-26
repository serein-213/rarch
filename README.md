# 归藏 (rarch)

[English](README.md) | [简体中文](README_ZH.md)

> **The Robust File Organizer** — A blazing fast, content-aware, and atomic file organization tool written in Rust.

[![Build Status](https://img.shields.io/badge/status-active-brightgreen.svg)]()
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)]()
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)]()

## Why rarch?

Most file organizers just move files by extension. **rarch** is designed for power users who care about data integrity, storage efficiency, and zero-latency organization.

### Visuals

![rarch UI](assets/rarch_tui.png)
*rarch Interactive TUI Dashboard*

![rarch CLI](assets/rarch_cli.png)
*rarch CLI Running in Dry-Run Mode*

### Key Features

- **Blazing Fast**: Powered by Rust and `rayon` for parallel processing. Scan and organize 100k+ files in seconds.
- **Atomic Undo**: Every operation is journaled. If you mess up your rules, `rarch undo` restores everything exactly where it was.
- **Content-Aware**: Don't be fooled by extensions. rarch uses deep magic-number inspection to identify file types (e.g., identifies a `.txt` as a `.png`).
- **Intelligent Deduplication**: Automatically detects identical files using SHA-256 and converts duplicates into **hard links**, saving GBs of disk space instantly.
- **Real-time Watch Mode**: Run `rarch watch` and never organize your Downloads folder again. It handles files the moment they arrive.
- **Interactive TUI**: A beautiful dashboard for those who prefer a keyboard-driven visual experience.

## Installation

```bash
cargo install rarch --features ui
```

## Usage

### ⚙️ 1. Configure

Create `rarch.toml`:

```toml
[[rules]]
name = "Photos"
extensions = ["jpg", "png", "heic"]
target = "Pictures/Sorted"

[[rules]]
name = "Documents"
extensions = ["pdf", "docx"]
target = "Documents/Work"
```

### 🛠️ 2. Organize

```bash
# Preview changes first
rarch run --dry-run

# Execute organization & deduplication
rarch run --path ~/Downloads
```

### 🕒 3. Undo

```bash
rarch undo
```

### 📡 4. Set it and forget it

```bash
rarch watch --path ~/Downloads
```

## Architecture

1. **Scanner**: Deep or shallow directory traversal.
2. **Engine**:
    - Parallel hashing (SHA-256).
    - Content-type inference.
    - Link-based deduplication logic.
3. **Journal**: JSON-based transaction log for 100% reliable undo.
4. **UI**: Zero-dependency TUI powered by `ratatui`.

## License

MIT OR Apache-2.0
