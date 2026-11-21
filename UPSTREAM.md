# 上游与同步说明 / Upstream and Sync

本文件记录上游项目的地址、本仓库与上游的关系、后续同步的来源与步骤，以及同步时容易冲突的地方。

This file records the upstream project, the relation between this repository and upstream, where
future changes come from, how to sync, and where conflicts are likely.

## 上游项目 / The upstream

| | |
| --- | --- |
| 名称 | Winc |
| 地址 | <https://github.com/DEEIX-AI/Winc> |
| 默认分支 | `dev`（上游在 `dev` 上开发，发布走 tag） |
| 主页 | <https://winc.si> |
| 许可 | Apache License 2.0（见 `LICENSE` 与 `NOTICE`） |
| 技术栈 | Rust 2024（`rust-toolchain.toml` 固定 1.99.1）、winit、自研 CPU 光栅化器 |

上游的提交历史原样保留在本仓库中：两个仓库共享同一段祖先提交，因此 `git merge upstream/dev`
能算出准确的三方合并结果。

## 远程配置 / Remotes

| 远程 | 地址 | 用途 |
| --- | --- | --- |
| `origin` | `https://github.com/xinggaoya/winc.git` | 本仓库，推送我们的改动 |
| `upstream` | `https://github.com/DEEIX-AI/Winc.git` | 上游官方仓库，只拉取不推送 |

核对与修复：

```bash
git remote -v
git remote set-url origin  https://github.com/xinggaoya/winc.git
git remote set-url upstream https://github.com/DEEIX-AI/Winc.git
```

## 分支约定 / Branches

| 分支 | 说明 |
| --- | --- |
| `main` | 本仓库的主分支，日常开发与推送都在这里 |
| `upstream/dev` | 上游的 `dev`（上游的默认分支），由 `git fetch upstream` 更新，不直接提交 |

上游在 `dev` 上开发、发布走 tag；本仓库改用自己的 `main`，与上游的 `dev` 一一对应地同步：
上游的改动进到本仓库的 `main`，本仓库的改动不推回上游。

## 同步步骤 / How to sync

```bash
git fetch upstream                      # 取回上游的新提交
git log --oneline HEAD..upstream/dev    # 看有哪些新提交
git merge upstream/dev                  # 并进来（本地有改动时先提交或 stash）
cargo fmt --all                         # 上游与本地都可能改动同一处，先统一格式
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                  # 跑一遍全部测试
git push origin main                    # 推送
```

上游合并进来后若测试不过，多半是下面列出的高发区；逐处对照上游的改动修好再提交。

### 提交前检查 / Before pushing

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask test --all
```

上游的 CI 同样跑这几项，格式与 clippy 的告警都会让 CI 失败。

## 改动的高发区 / Where conflicts are likely

上游的开发节奏很快，下面这些地方我们改动最多，同步时最可能冲突：

| 位置 | 我们的改动 | 冲突时怎么办 |
| --- | --- | --- |
| `crates/winc-app/src/layout.rs` | 三栏布局：左右两栏的矩形、各面板的槽位、右栏分隔条 | 以我们的布局为准，同时保留上游新增的布局字段 |
| `crates/winc-app/src/app/workspace/columns.rs` | 面板属于哪一栏、焦点、右栏开关 | 保留上游新增的字段，其余按我们的模型改 |
| `crates/winc-app/src/app/workspace/switch.rs`、`workspaces.rs` | 工作区登记表与切换逻辑 | 上游若新增与目录相关的状态，记得一并纳入 `WorkspaceSlot` |
| `crates/winc-app/src/app/workspace/bar.rs`、`place.rs` | 顶栏的工作区切换条 | 上游若改动标签栏的起点，两处都要跟着改 |
| `crates/winc-app/src/app/settings/panels.rs`、`layout/settings/panels.rs` | 「面板」设置页 | 上游若新增设置项或改设置页布局，两边都要对齐 |
| `crates/winc-app/src/app/git/commit.rs`、`view/git/commit/**` | 提交面板改用自己所在栏的卡片 | 提交视图上游改动频繁，冲突时逐处确认裁剪区域 |
| `crates/winc-app/src/app/input/mouse/target.rs` | 标题栏控件登记为客户区 | 上游若新增顶栏控件，同样要登记，否则在 Windows 上点不动 |
| `crates/winc-editor/src/lib.rs` | 文档编号改为进程内全局唯一 | 与上游的 `next_id` 冲突时保留全局计数器 |
| `crates/winc-app/src/updates.rs`、`updates/github.rs` | 自动更新改用 GitHub 的发布 | 上游若改动 appcast 的解析，两边都要保留 |
| `crates/winc-config/src/config/panels.rs`、`config/fields.rs` | 面板种类与 `panels` 一节的配置 | 上游若新增配置节，追加到 `SECTIONS` 表即可 |
| `crates/winc-i18n/locales/**` | 新增文案 | 上下游的文案 id 可能撞车，加前缀或换措辞 |

### 合并冲突的取舍 / Which side wins

- **界面结构**（布局、面板、工作区）以本仓库为准：这些是我们二次开发的核心，上游没有对应概念。
- **功能细节**（语法高亮、预览、Git 操作、性能优化）以上游为准：这些是上游的主战场，我们没有改动。
- 拿不准时，先看上游的提交说明，再回到本文件的高发区表核对。

## 仓库结构 / Repository layout

上游的目录结构原样保留，不做重排——重排会让每一次同步都变成全文件冲突。二次开发新增的文件都在
`crates/winc-app/src/` 与 `crates/winc-config/src/` 下面，与上游的目录约定一致：

```
crates/
  winc-app/src/
    app/workspace/        工作区、左右两栏、工作区切换条
    app/settings/         设置页的操作（含「面板」页）
    layout/               布局计算（含三栏与面板槽位）
    view/                 绘制（含左右两栏与面板设置页）
  winc-config/src/config/ 配置项（含面板种类与 panels 一节）
UPSTREAM.md               本文件
CHANGELOG.md              变更记录
README.md / README.zh-CN.md
```

## 发布 / Releasing

本仓库自行发布：改动并入 `main`、验证通过后，抬高 `VERSION` 文件里的版本号，打一个 tag
（例如 `v0.1.900`），推送 tag。推送 tag 会触发 Release 工作流：

1. `release.yml` 收到 tag，取出源码并算出这次发布要用的版本号。
2. 四个平台各自构建：macOS（Apple silicon 与 Intel）、Linux（x86_64）、Windows（x86_64）。
3. 四个平台的产物都通过核对后，`release.yml` 把它们、校验和清单 `SHA256SUMS` 与发布说明一起
   上传到 GitHub 的 Release（直接发布，不再是草稿）。

发布说明由 Git 日志生成：`release/common/notes.mjs` 读上一次 tag 到这次 tag 之间的提交，
按约定的提交类型（`feat`、`fix`、`docs` 等）分组，每条提交一行，附在下载表格后面。

签名与公证按上游的做法，需要仓库的密钥：没有配置时 macOS 只做临时签名、Windows 不签名，
构建照常完成。应用内更新的签名密钥是仓库的机密 `WINC_UPDATE_PRIVATE_KEY`（Sparkle 的
Ed25519 种子，base64）。

程序内更新直接用 GitHub 的 Release：应用问仓库的发布订阅源（`releases.atom`）要最新的版本号，
从发布里下载附件，用 `SHA256SUMS` 里的校验值核对。appcast 通道仍在，取不到发布时退回它。

```bash
# 发一个版本
echo 0.1.900 > VERSION
git add VERSION && git commit -m "chore: 发布 0.1.900"
git tag -a v0.1.900 -m "Release v0.1.900"
git push origin main v0.1.900     # 推送 tag 即开始构建与发布
```

tag 必须是 `v<版本号>`，且与 `VERSION` 文件里的版本号一致，否则发布脚本会拒绝（见
`release/README.md`）。若想让工作流代替你打 tag，手动运行 “Release Tag” 工作流即可。
