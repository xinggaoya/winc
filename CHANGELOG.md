# 变更记录 / Changelog

本文件记录本仓库（二次开发）相对上游的改动。上游自己的发布说明见
[上游 Releases](https://github.com/DEEIX-AI/Winc/releases) 与应用内的更新日志。

This file records the changes of this repository relative to upstream. The upstream's own release
notes live in [upstream Releases](https://github.com/DEEIX-AI/Winc/releases) and in the app.

记录格式与上游一致：一条一行，动词开头，说明做了什么。

## 未发布 / Unreleased

### 新增 / Added

- 多工作区模式：同一个窗口里同时打开多个项目，顶栏里列出各个项目，随时切换；每个工作区各自保留
  标签、文件树的展开状态、Git 状态与终端会话。快捷键 `⌘⌃→` / `⌘⌃←` 切换，`⌘⌃W` 关闭当前工作区
  （Windows/Linux：`Ctrl+Alt+N` / `Ctrl+Alt+P` / `Ctrl+Alt+W`）。
- 三栏布局：左栏放文件树等面板，中间一栏固定是标签栏、编辑区与终端，右栏放 Git 提交视图等面板；
  右栏可用窗口右上角的面板按钮收起或展开，宽度可拖动分隔条调整。
- 「面板」设置页：拖动行首的把手把面板拖到另一栏或换个位置，用行尾的按钮上移、下移、移到另一栏、
  隐藏；右栏的宽度也能在这里一档一档地调。
- 配置项 `panels.left`、`panels.right`（面板种类数组）、`panels.rightWidth`（右栏宽度）与
  `multiWorkspace`（多工作区模式）。
- 设置项「多工作区模式」在「面板」页里，与面板排布放在一起。

### 改动 / Changed

- 侧栏的视图切换控件（文件、搜索、提交三个分段）换成左右两栏里的面板：每一栏里放哪些面板、各自的
  顺序由配置决定，中间一栏不参与配置。
- 打开另一个目录不再关闭当前目录，而是把它留成另一个工作区；关掉最后一个工作区时行为与原来一致。
- 文档编号改为进程内全局唯一，多个工作区同时打开时编号不会重复。
- 自动更新改用 GitHub 的发布：版本号取自仓库的发布订阅源，附件从发布里下载，用发布里的
  `SHA256SUMS` 核对；构建时可用 `WINC_UPDATE_REPOSITORY` 改仓库，appcast 通道仍在，取不到发布时
  退回它。
- 本仓库自行发布：推送 tag 即触发四个平台的构建，产物与发布说明（由 Git 日志生成）一起发到
  GitHub 的 Release。

### 修复 / Fixed

- 左右两栏里的面板可以正常点击与悬停：提交视图的命中判断与裁剪区域改用面板自己所在栏的位置。
- 右栏的开关按钮在 Windows 上可点：顶栏由系统处理，其中的控件要登记为客户区，否则点击被当成
  拖动窗口。
- 顶栏里的工作区切换按钮与「打开项目」按钮同样登记为客户区，在 Windows 上可点。
- 文件面板放到右栏时，文件树、大纲随面板一起挪过去，不再留在左栏。
- 多工作区模式下断开连接，关掉的是当前这个工作区，其余的还开着；换到另一个工作区时，终端面板
  开着就还开着。
- 换到不是仓库的目录时，焦点从提交面板回到文件树：否则键入的文字会进入看不见的输入框。

## 与上游的关系 / Relation to upstream

| | |
| --- | --- |
| 上游 | <https://github.com/DEEIX-AI/Winc>（分支 `dev`） |
| 本仓库 | <https://github.com/xinggaoya/winc>（分支 `main`） |
| 本次改动所基于的上游提交 | `16a7785f`（`Merge pull request #336 from DEEIX-AI/support_latex`） |

本仓库保留了上游的完整历史，同步方式与冲突处理见 [UPSTREAM.md](./UPSTREAM.md)。

## 上游改动带来的记录 / Changes carried over from upstream

同步上游时，以下上游改动一并进入本仓库（按上游的提交说明摘录）：

- `01309f12` 预览 LaTeX 文档，并在 Markdown 中渲染 LaTeX 公式。
- `43a2e692` 版本号升到 v0.1.852。
