<p align="center">
  <img src="./crates/winc-app/assets/icon.svg" alt="winc" width="128" />
</p>

<h1 align="center">Winc · Secondary Development</h1>

<p align="center">
  A lighter, faster, more comfortable desktop editor — with multi-workspace mode, a three-column
  layout, and a configurable panel arrangement on top of upstream.
</p>

<p align="center">
  English | <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://winc.si"><img alt="Website" src="https://img.shields.io/badge/Website-winc.si-black" /></a>
  <a href="./LICENSE"><img alt="License" src="https://img.shields.io/badge/License-Apache%202.0-blue" /></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-b7410e" />
  <img alt="Platforms" src="https://img.shields.io/badge/Platforms-macOS%20%7C%20Linux%20%7C%20Windows-555" />
</p>

<p align="center">
  <a href="https://winc.si/#download"><img alt="Download Winc" src="./.github/assets/download-en.svg" /></a>
</p>

## What this is

This is a **secondary development branch** of the upstream project
[Winc](https://github.com/DEEIX-AI/Winc) — a desktop editor written in Rust with its own rendering
engine: it starts in 0.1 seconds and uses just 52 MB of memory with 100,000 lines open. Multiple
cursors, find and replace, and side-by-side diffs are all there, and every keystroke and scroll
responds instantly.

This repository adds three interface changes on top of upstream (see
[Secondary development](#secondary-development)):

1. **Multi-workspace mode** — run several projects in one window and switch between them freely;
2. **A three-column layout** — files on the left, editor and terminal in the fixed middle column,
   Git and other panels on the right;
3. **A configurable panel arrangement** — drag panels between the columns, reorder them, and have
   the arrangement persisted in the configuration.

The upstream address, the sync workflow, and the change records live in
**[UPSTREAM.md](./UPSTREAM.md)** and **[CHANGELOG.md](./CHANGELOG.md)**.

| Repository | Address |
| --- | --- |
| This repository (secondary development) | <https://github.com/xinggaoya/winc> |
| Upstream project | <https://github.com/DEEIX-AI/Winc> |
| Upstream website | <https://winc.si> |

## Secondary development

### 1. Multi-workspace mode

Several projects live in one window at the same time. Each one keeps its own state — open tabs, the
expanded state of the file tree, the Git status, and the terminal session — and switching back finds
everything as it was.

- The projects open in the window are listed at the left of the top bar; click one to switch. The
  folder button at the end opens another project.
- Opening a folder no longer closes the previous one: it stays as another workspace (turn the mode
  off in the settings to get the old behaviour back).
- Closing a project closes only that one; the others keep running.
- Shortcuts: `⌘⌃→` / `⌘⌃←` switch to the next and previous workspace, `⌘⌃W` closes the current one
  (`Ctrl+Alt+N` / `Ctrl+Alt+P` / `Ctrl+Alt+W` on Windows and Linux).

### 2. A three-column layout

The window is split into three columns:

| Column | Contents |
| --- | --- |
| Left | The file tree and other panels, with the project name and menu on top |
| Middle (fixed) | The tab bar, the editor, and the terminal |
| Right | The Git commit view and other panels |

- The panel button at the top right of the window collapses and expands the right column. The button
  stays where it is, so it can be pressed again to bring the column back.
- The divider on the left edge of the right column resizes it.
- The middle column is not configurable: the editor and the terminal always stay together.

### 3. A configurable panel arrangement

The **Panels** page of the settings (`⌘,` → Panels) can:

- drag the `≡` handle at the start of a row to move a panel to another column or another position;
- move a panel up or down, send it to the other column, or hide it with the buttons at the end of
  the row;
- change the width of the right column.

The arrangement is written to the configuration file (see [Configuration](#configuration)) and is
restored on the next start.

### Configuration

The `panels` section of the configuration file says which panels are in which column and in what
order; `multiWorkspace` turns multi-workspace mode on and off:

```jsonc
{
  "panels": {
    "left": ["files"],              // left column: the file tree
    "right": ["commit", "search"],  // right column: the commit view, the search
    "rightWidth": 320               // width of the right column (180–720)
  },
  "multiWorkspace": true           // multi-workspace mode (on by default)
}
```

Panel names: `files` (file tree), `search` (project-wide search), `commit` (the Git commit view).

## Overview

winc is built with Rust and its own rendering engine: it starts in 0.1 seconds and uses just 52 MB
of memory with 100,000 lines open. Multiple cursors, find and replace, and side-by-side diffs are
all there, and every keystroke and scroll responds instantly.

| Measure | Result |
| --- | --- |
| Cold start to first frame | 119 ms (median of 20 runs) |
| Resident memory with 100,000 lines open | 52 MB |
| A 1 GB file, editable and highlighted | 220 ms |

Measured on macOS; results vary with hardware and system load.

## Features

| Area | Capabilities |
| --- | --- |
| Editing | Multiple cursors, find and replace, native input methods, smooth scrolling in gigabyte-sized files, and an experimental Vim mode |
| Syntax | Highlighting for common programming, markup, data, and configuration languages, plus lightweight go-to-definition and usages without a language server |
| Formatting | Whole-file formatting with external tools such as rustfmt, Prettier, Black, Ruff, and gofmt |
| Preview | Markdown with math and Mermaid diagrams, images, SVG, PDF (macOS and Windows), Word documents (docx), and spreadsheets (xlsx, csv), with the source one click away |
| Git | Side-by-side diffs with word-level highlights, staging by hunk, commit and push, a commit graph, and blame |
| Project | A file tree, quick open, and project search that follows `.gitignore` |
| Terminal | An integrated terminal with automatic loading of project-local Python environments |
| Remote | Experimental SSH remote: open a folder on a Linux host and edit, search, format, use Git, and open terminals there. On Windows, open folders inside WSL distributions |
| Interface | Sixteen built-in themes, light and dark, and a complete interface in English and Simplified Chinese |
| Modules | Git, the terminal, and SSH remote can each be turned off. First-time setup on the start page offers the appearance, theme, language, and modules |
| Updates | Signed in-app updates on all three platforms, with the release notes shown in the app after each update |

Touchscreens support tapping buttons and placing the caret, double-tapping to select a word,
swiping to scroll, and dragging scrollbars and panel dividers. Hold for 500 ms to open the context
menu. Swipes stay in the region where they started and stop on release. Additional fingers cancel
the current gesture; lift all fingers before starting another. On Windows, drag the title bar to
move the window, double-tap it to maximize or restore, and drag the window edges to resize.
Window dragging and edge snapping use the same native Windows behavior as mouse input.

Image and SVG previews zoom like the PDF preview: hold Ctrl (⌘ on macOS) and scroll, pinch, or
press Ctrl/⌘ with `+`, `-` or `0`; the point under the pointer stays in place. Scroll or drag a
zoomed image to pan, and double-click to fit it again. This also works in the SVG split preview.

## Platforms

| Platform | Requirements | Package |
| --- | --- | --- |
| macOS | macOS 11 or later, Apple silicon or Intel | Disk image |
| Linux | x86_64, X11 or Wayland | AppImage, deb, Arch repository |
| Windows | Windows 10 1809 or later, x86_64 | NSIS, MSI, portable ZIP |

The portable ZIP keeps settings and state in the `data` folder beside `winc.exe`; delete that
folder to keep them in the user profile instead.

winc is developed and tested mainly on macOS. Download it from
[winc.si](https://winc.si/#download); all versions are on
[GitHub Releases](https://github.com/DEEIX-AI/Winc/releases).

## Command Line

```bash
winc .                  # open the current folder
winc file.rs:12:5       # open a file at a line and column
winc --wait file        # wait until the tab closes, for example as the Git editor
cargo build 2>&1 | winc -   # read standard input
```

The Windows installers and the Linux deb and Arch packages add `winc` to `PATH`; the Windows portable ZIP leaves `PATH` unchanged. On macOS, choose **Winc → Install winc Command…** from the menu bar or run **Shell Command: Install winc Command** from the command palette; with the Linux AppImage, use the command palette.

On macOS the app is started by the system, so it does not inherit the environment variables of your shell. Use `winc --new-instance` to keep them.

## Building from Source

winc builds with Rust 1.99.1; `rust-toolchain.toml` selects the toolchain automatically. On Windows
the [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) are needed as well
(the "Desktop development with C++" workload, with MSVC v143 and the Windows SDK).

```bash
cargo run --release                 # build and run
cargo run --release -- <path>       # open a file or folder
```

For versioning, signing configuration and release workflows, see [Releasing winc](release/README.md).

## Development

```bash
cargo xtask test                    # test the crates changed since HEAD
cargo xtask test --all              # test the whole workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo xtask bench                   # measure startup, memory, and scrolling against budgets
```

The interface is written in Rust. Windows come from winit, and rendering is done by winc's own
CPU rasterizer. On macOS, frames are written directly to an IOSurface.

The workspace is split into crates by responsibility:

| Crate | Responsibility |
| --- | --- |
| `winc-core` | Text, editing, undo, and diffing, independent of the interface |
| `winc-syntax` | Lexers for syntax highlighting |
| `winc-editor` | Editor logic: documents, file tree, search, Git, and formatting |
| `winc-vim` | Vim mode without the interface: key notation, the command table, parsing, and evaluation |
| `winc-backend` | File system access for local paths and for paths on remote hosts |
| `winc-server` | The server process that runs on a remote host and serves file operations over SSH |
| `winc-watch` | Watches folders on the machine that holds them (WSL distributions, remote hosts) |
| `winc-render` | Rendering: glyphs, rasterization, and presentation |
| `winc-preview` | Previews for images, SVG, Markdown, PDF, Word documents, and spreadsheets |
| `winc-typeset` | Layout for the Markdown preview |
| `winc-pty` | Terminal emulation and the pseudo-terminal helper process |
| `winc-config` | Configuration and state files |
| `winc-i18n` | Interface strings (Fluent bundles) |
| `winc-version` | The product version, read from the `VERSION` file |
| `winc-app` | The application: windows, events, and views |
| `winc` | The executable's entry point, which only calls into `winc-app` |
| `xtask` | Development automation: tests, benchmarks, and packaging |

The secondary development is concentrated in these places:

| Location | Contents |
| --- | --- |
| `crates/winc-config/src/config/panels.rs` | The panel kinds and the default arrangement of the two columns |
| `crates/winc-app/src/layout/columns.rs` | The geometry of the panels inside a column |
| `crates/winc-app/src/app/workspace/columns.rs` | Which column a panel is in, the focus, and the right column's switch |
| `crates/winc-app/src/app/workspace/workspaces.rs` | The workspace registry and the state of each workspace |
| `crates/winc-app/src/app/workspace/switch.rs` | Switching, closing, and adding workspaces |
| `crates/winc-app/src/app/workspace/bar.rs` | The workspace switcher in the top bar |
| `crates/winc-app/src/app/settings/panels.rs` | The actions of the Panels settings page |
| `crates/winc-app/src/layout/settings/panels.rs` | The geometry of the Panels settings page |
| `crates/winc-app/src/view/workspace/columns.rs` | Drawing the two side columns |

## Syncing with upstream

This repository keeps the upstream history. The `upstream` remote points at the official repository
and `origin` at this one. The sync steps, the areas where conflicts are likely, and the branch
conventions are in **[UPSTREAM.md](./UPSTREAM.md)**.

```bash
git remote -v                       # origin: this repository; upstream: DEEIX-AI/Winc
git fetch upstream
git merge upstream/dev              # bring the upstream changes in
```

## Contributing

The upstream contribution guide is in [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md); the
upstream project develops on the `dev` branch, this repository on `main`. The changes here are
described in the commit messages and in [CHANGELOG.md](./CHANGELOG.md).

## Contact

- Upstream website: <https://winc.si>
- Upstream email: <support@deeix.com>
- Telegram: <t.me/deeix_chat>
- X: [@DEEIX_AI](https://x.com/DEEIX_AI)

## Links

- This repository: <https://github.com/xinggaoya/winc>
- Upstream project: <https://github.com/DEEIX-AI/Winc>
- [Rust](https://www.rust-lang.org)
- [DEEIX AI](https://deeix.com)
- [LINUX DO](https://linux.do)

## License

winc is licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE). This
repository is a secondary development of the upstream project, keeps the same license, and preserves
the upstream copyright notices.
