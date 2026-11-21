<p align="center">
  <img src="./crates/winc-app/assets/icon.svg" alt="winc" width="128" />
</p>

<h1 align="center">Winc · 二次开发版</h1>

<p align="center">
  更轻、更快、更顺手的桌面编辑器 —— 在上游基础上加入多工作区模式、三栏布局与可自定义的面板排布。
</p>

<p align="center">
  <a href="./README.md">English</a> | 简体中文
</p>

<p align="center">
  <a href="https://winc.si"><img alt="Website" src="https://img.shields.io/badge/Website-winc.si-black" /></a>
  <a href="./LICENSE"><img alt="License" src="https://img.shields.io/badge/License-Apache%202.0-blue" /></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-b7410e" />
  <img alt="Platforms" src="https://img.shields.io/badge/Platforms-macOS%20%7C%20Linux%20%7C%20Windows-555" />
</p>

<p align="center">
  <a href="https://winc.si/#download"><img alt="下载 Winc" src="./.github/assets/download-zh.svg" /></a>
</p>

## 这是什么

这是 [上游项目 Winc](https://github.com/DEEIX-AI/Winc) 的**二次开发分支**。上游是 Rust 与自研渲染引擎
写成的桌面编辑器：0.1 秒启动，十万行代码仅占 52 MB 内存，多光标、查找替换、并排差异都在，每一次
输入与滚动都即时响应。

本仓库在上游代码之上做了三项界面改动（详见 [二次开发内容](#二次开发内容)）：

1. **多工作区模式** —— 同一个窗口里同时打开多个项目，随时切换；
2. **三栏布局** —— 左侧目录、中间文件与终端（固定）、右侧 Git 等面板；
3. **可自定义的面板排布** —— 面板放哪一栏、什么顺序都能拖动调整，并写进配置。

上游地址、后续同步方式与变更记录见 **[UPSTREAM.md](./UPSTREAM.md)** 与
**[CHANGELOG.md](./CHANGELOG.md)**。

| 项目 | 地址 |
| --- | --- |
| 本仓库（二次开发） | <https://github.com/xinggaoya/winc> |
| 上游项目 | <https://github.com/DEEIX-AI/Winc> |
| 上游主页 | <https://winc.si> |

## 二次开发内容

### 1. 多工作区模式

一个窗口里同时打开多个项目，每个项目是一份独立的"现场"：打开的标签、文件树的展开状态、Git 状态、
终端会话都各自保留，切过去时原样还在。

- 当前打开的各个项目是左栏顶部的一排标签：当前打开的那个填实底，其余淡一些；点一下就切过去，
  鼠标移上去可以关掉它，按住可以拖动调整顺序。
- 切换目录不再关闭原来的目录，而是把它留成另一个工作区（可在设置里关闭该模式，恢复旧行为）。
- 关闭项目时只关这一个，其余工作区不受影响。
- 快捷键：`⌘⌃→` / `⌘⌃←` 切换下一个、上一个工作区，`⌘⌃W` 关闭当前工作区
  （Windows/Linux 为 `Ctrl+Alt+N` / `Ctrl+Alt+P` / `Ctrl+Alt+W`）。

### 2. 三栏布局

窗口分成三栏：

| 栏 | 内容 |
| --- | --- |
| 左栏 | 文件树等面板，顶部是工作区标签与项目菜单 |
| 中间（固定） | 标签栏、编辑区、终端 |
| 右栏 | Git 提交视图等面板 |

- 三张卡片两侧的间距一致：左栏与中间一栏之间、中间一栏与右栏之间都隔同样宽。

- 右栏可用窗口右上角的面板按钮收起或展开，按钮位置固定，收起后仍在原地。
- 右栏左边缘的分隔条可以拖动调整宽度。
- 中间一栏不参与配置，编辑区与终端始终在一起。

### 3. 可自定义的面板排布

设置里的**「面板」**页（`⌘,` → 面板）里可以：

- 拖动行首的 `≡` 把手，把面板拖到另一栏或换个位置；
- 用行尾的按钮上移、下移、移到另一栏、隐藏或重新显示；
- 调整右栏宽度。

排布写进配置文件（见 [配置项](#配置项)），下次启动照旧。

### 配置项

在配置文件的 `panels` 一节里指定左右两栏的内容与顺序，`multiWorkspace` 控制多工作区模式：

```jsonc
{
  "panels": {
    "left": ["files"],            // 左栏：文件树
    "right": ["commit", "search"], // 右栏：提交视图、搜索
    "rightWidth": 320             // 右栏宽度（180–720）
  },
  "multiWorkspace": true         // 多工作区模式（默认开）
}
```

面板的取值：`files`（文件树）、`search`（全局搜索）、`commit`（Git 提交视图）。

## 概述

winc 以 Rust 与自研渲染引擎构建：0.1 秒启动，十万行代码仅占 52 MB 内存。多光标、查找替换、并排差异，
每一次输入与滚动都即时响应。

| 指标 | 结果 |
| --- | --- |
| 冷启动到第一帧 | 119 ms（20 次的中位数） |
| 打开 10 万行时的常驻内存 | 52 MB |
| 1 GB 的文件，可编辑、有高亮 | 220 ms |

在 macOS 上测得，随硬件和系统负载略有不同。

## 功能

| 领域 | 能力 |
| --- | --- |
| 编辑 | 多光标、查找替换、原生输入法、GB 级文件的平滑滚动，还有实验性的 Vim 模式 |
| 语法 | 常见编程、标记、数据与配置语言的高亮，以及不依赖语言服务器的轻量跳转定义与查找引用 |
| 格式化 | 用 rustfmt、Prettier、Black、Ruff、gofmt 等外部工具整文件格式化 |
| 预览 | Markdown（含公式与 Mermaid）、图片、SVG、PDF（macOS 与 Windows）、Word（docx）、表格（xlsx、csv），源码一键可切 |
| Git | 逐词高亮的并排差异、按块暂存、提交与推送、提交图与逐行追责 |
| 项目 | 文件树、快速打开、遵循 `.gitignore` 的项目搜索 |
| 终端 | 内置终端，自动加载项目本地的 Python 环境 |
| 远程 | 实验性的 SSH 远程：在 Linux 主机上打开目录并编辑、搜索、格式化、使用 Git、开终端；Windows 下还能打开 WSL 发行版里的目录 |
| 界面 | 十六套内置主题、明暗两色，完整的英文与简体中文界面 |
| 模块 | Git、终端、SSH 远程各自可关；开始页的首次设置里有外观、主题、语言与模块 |
| 更新 | 三个平台都有带签名的应用内更新，更新后显示更新日志 |

触摸屏支持点按按钮与定位光标、双击选词、滑动滚动、拖动滚动条与面板分隔条；长按 500 毫秒弹出上下文
菜单。滑动不越出起始区域，抬手即停；多指操作会取消当前手势，抬起所有手指后才能开始下一次。在 Windows
上拖动标题栏移动窗口、双击最大化或还原、拖动窗口边缘改变大小，窗口拖动与贴边使用与鼠标输入相同的
原生行为。

图片与 SVG 预览的缩放方式与 PDF 预览一致：按住 Ctrl（macOS 上是 ⌘）滚动、双指捏合，或按 Ctrl/⌘ 加
`+`、`-`、`0`，指针下的位置保持不动。滚动或拖动可平移已放大的图片，双击回到适合窗口的大小；SVG 的
分栏预览同样如此。

## 平台

| 平台 | 要求 | 安装包 |
| --- | --- | --- |
| macOS | macOS 11 及以上，Apple 芯片或 Intel | 磁盘镜像 |
| Linux | x86_64，X11 或 Wayland | AppImage、deb、Arch 仓库 |
| Windows | Windows 10 1809 及以上，x86_64 | NSIS、MSI、绿色版 ZIP |

绿色版 ZIP 把设置与状态放在 `winc.exe` 旁边的 `data` 目录里；删掉该目录即改回存到用户目录。

winc 主要在 macOS 上开发与测试。本仓库自行发布：版本在
[GitHub Releases](https://github.com/xinggaoya/winc/releases)，发布说明由两次发布之间的提交记录
生成。

## 命令行

```bash
winc .                  # 打开当前目录
winc file.rs:12:5       # 打开文件并定位到某行某列
winc --wait file        # 等标签关闭后再退出，例如作为 Git 的编辑器
cargo build 2>&1 | winc -   # 读标准输入
```

Windows 安装包与 Linux 的 deb、Arch 包会把 `winc` 加进 `PATH`；Windows 绿色版 ZIP 不改 `PATH`。
macOS 上从菜单栏选 **Winc → Install winc Command…**，或在命令面板里运行 **Shell Command: Install
winc Command**；Linux AppImage 用命令面板。

macOS 上应用由系统启动，因此不继承 shell 的环境变量。用 `winc --new-instance` 保留它们。

## 从源码构建

winc 用 Rust 1.99.1 构建，`rust-toolchain.toml` 会自动选择工具链。Windows 上还需要
[Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)（"使用 C++ 的桌面开发"
工作负载，含 MSVC v143 与 Windows SDK）。

```bash
cargo run --release                 # 构建并运行
cargo run --release -- <path>       # 打开文件或目录
```

版本号、签名配置与发布流程见 [Releasing winc](release/README.md)；本仓库的发布办法（抬高
`VERSION`、打 tag、推送 tag）与上游同步办法见 [UPSTREAM.md](UPSTREAM.md)。

## 开发

```bash
cargo xtask test                    # 测试与 HEAD 相比有改动的 crate
cargo xtask test --all              # 测试整个工作区
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo xtask bench                   # 对照预算测启动、内存与滚动
```

界面用 Rust 写。窗口来自 winit，渲染由 winc 自研的 CPU 光栅化器完成；macOS 上帧直接写进 IOSurface。

工作区按职责拆成若干 crate：

| crate | 职责 |
| --- | --- |
| `winc-core` | 文本、编辑、撤销与差异，与界面无关 |
| `winc-syntax` | 语法高亮的词法分析 |
| `winc-editor` | 编辑器逻辑：文档、文件树、搜索、Git 与格式化 |
| `winc-vim` | 不含界面的 Vim 模式：键位记法、命令表、解析与求值 |
| `winc-backend` | 本地路径与远端主机路径的文件访问 |
| `winc-server` | 在远端主机上运行、通过 SSH 提供文件操作的服务进程 |
| `winc-watch` | 监视文件所在的机器（WSL 发行版、远端主机）上的目录 |
| `winc-render` | 渲染：字形、光栅化与呈现 |
| `winc-preview` | 图片、SVG、Markdown、PDF、Word 与表格的预览 |
| `winc-typeset` | Markdown 预览的排版 |
| `winc-pty` | 终端模拟与伪终端辅助进程 |
| `winc-config` | 配置文件与状态文件 |
| `winc-i18n` | 界面文案（Fluent 文案包） |
| `winc-version` | 产品版本，读自 `VERSION` 文件 |
| `winc-app` | 应用本体：窗口、事件与视图 |
| `winc` | 可执行文件入口，只调用 `winc-app` |
| `xtask` | 开发自动化：测试、基准与打包 |

本次二次开发新增或改动的部分集中在：

| 位置 | 内容 |
| --- | --- |
| `crates/winc-config/src/config/panels.rs` | 面板种类与左右两栏的默认排法 |
| `crates/winc-app/src/layout/columns.rs` | 三栏布局里各面板的几何 |
| `crates/winc-app/src/app/workspace/columns.rs` | 面板属于哪一栏、焦点与右栏开关 |
| `crates/winc-app/src/app/workspace/workspaces.rs` | 工作区登记表与各自的现场 |
| `crates/winc-app/src/app/workspace/switch.rs` | 工作区的切换、关闭与新增 |
| `crates/winc-app/src/app/workspace/bar.rs` | 左栏顶部的工作区标签条 |
| `crates/winc-app/src/app/settings/panels.rs` | 「面板」设置页的操作 |
| `crates/winc-app/src/layout/settings/panels.rs` | 「面板」设置页的几何 |
| `crates/winc-app/src/view/workspace/columns.rs` | 左右两栏的绘制 |

## 与上游同步

本仓库保留上游的历史，`upstream` 远程指向官方仓库，`origin` 指向本仓库。同步步骤、冲突高发区与
分支约定见 **[UPSTREAM.md](./UPSTREAM.md)**。

```bash
git remote -v                       # origin: 本仓库；upstream: DEEIX-AI/Winc
git fetch upstream
git merge upstream/dev              # 把上游的改动并进来
```

## 贡献

上游的贡献指南见 [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md)：上游项目在 `dev` 分支上开发，
本仓库在自己的 `main` 分支上开发。本仓库的改动以提交信息说明，见 [CHANGELOG.md](./CHANGELOG.md)。

## 联系方式

- 上游主页：<https://winc.si>
- 上游邮箱：<support@deeix.com>
- Telegram：<t.me/deeix_chat>
- X：[@DEEIX_AI](https://x.com/DEEIX_AI)

## 相关链接

- 本仓库：<https://github.com/xinggaoya/winc>
- 上游项目：<https://github.com/DEEIX-AI/Winc>
- [Rust](https://www.rust-lang.org)
- [DEEIX AI](https://deeix.com)
- [LINUX DO](https://linux.do)

## 许可

winc 按 [Apache License 2.0](LICENSE) 授权，详见 [NOTICE](NOTICE)。本仓库是上游的二次开发版本，
沿用同一许可，并保留上游的版权声明。
