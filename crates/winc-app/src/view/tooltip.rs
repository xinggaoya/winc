//! 悬停提示：停留满 `TOOLTIP_DELAY` 后淡入，按目标放在按钮下方、旁边或从鼠标处开始。

use super::*;
use crate::app::{OutlineTarget, ResultRow, SyncState};
use crate::layout::{PanelButton, PanelTool, RowAction, SearchButton};

/// 绘制按钮的悬停提示。
///
/// 鼠标在按钮上停留满 `TOOLTIP_DELAY`（500 毫秒）才显示，避免鼠标扫过时闪现。提示位于按钮
/// 下方并水平居中；下方放不下时（状态栏上的按钮）改在上方；概览标尺上的提示位于标尺左侧。
pub(super) fn draw_tooltip(app: &App, frame: &mut Frame<'_>) {
    // 鼠标刚离开的提示在原处淡出。
    if let Some((button, label, place, clock)) = &app.overlays.tooltip.leaving {
        draw_tip(app, frame, (*button, label, *place), clock.fade_out());
    }
    // 停留时间不足时直接返回，不拼接提示文字，也不查找命中框。
    let rested = app
        .overlays
        .tooltip
        .since
        .is_some_and(|since| since.elapsed() >= TOOLTIP_DELAY);
    if !rested {
        return;
    }
    let (button, label): (Rect, String) = match tooltip::tooltip(app) {
        Some(tip) => tip,
        None => match app.pointer.hover {
            Hover::Search(button) => (
                SearchParts::new(app.layout.search).rect(button),
                button.label(),
            ),
            Hover::TreeTool(tool) => match app.hits.tree_tools.iter().find(|(_, t)| *t == tool) {
                Some(&(button, _)) => (button, tool.label().to_owned()),
                None => return,
            },
            Hover::TreeMore => match app.hits.tree_more {
                Some(button) => (button, t!("workspace-tip-more-actions").to_owned()),
                None => return,
            },
            Hover::SidebarButton => (
                app.layout.sidebar_button,
                crate::layout::with_key(t!("cmd-toggle-sidebar"), Some(Action::ToggleSidebar)),
            ),
            Hover::Browser => (app.layout.browser, t!("preview-browser-open").to_owned()),
            Hover::Preview(mode) => {
                use winc_editor::PreviewMode::*;
                let [source, split, preview] = crate::layout::preview_segments(app.layout.preview);
                // 「预览」对应 ⇧⌘V（在源代码与预览之间切换）。
                match mode {
                    Source => (source, t!("preview-mode-source").to_owned()),
                    Split => (split, t!("preview-mode-split").to_owned()),
                    Preview => (
                        preview,
                        crate::layout::with_key(
                            t!("preview-mode-preview"),
                            Some(Action::TogglePreview),
                        ),
                    ),
                }
            }
            // 格式工具栏的按钮：名字和快捷键。
            Hover::SelectionTool(tool) => {
                let Some(&(rect, _)) = app.hits.selection_tools.iter().find(|(_, t)| *t == tool)
                else {
                    return;
                };
                (
                    rect,
                    crate::layout::with_key(tool.label(), Some(tool.action())),
                )
            }
            // Markdown 预览中的链接：提示地址，脚注的编号提示脚注的内容；可编辑的预览中另起一行
            // 说明怎样打开。
            Hover::MarkdownLink(i) | Hover::MarkdownLinkText(i)
                if !app.markdown_link_is_control(i) =>
            {
                let Some((rect, url)) = app.hits.markdown_links.get(i) else {
                    return;
                };
                let note = (url.strip_prefix("#fn-"))
                    .and_then(|n| app.preview.markdown.parsed.as_ref()?.footnote_text(n));
                let text = match note {
                    Some(note) => note,
                    None if url.starts_with("#fnref-") => return,
                    None => url.clone(),
                };
                // 无法打开的链接另起一行显示原因。
                let text = match app.preview.markdown.links.get(url) {
                    Some(broken) => format!("{text}\n{}", broken.reason()),
                    None => text,
                };
                let key = if cfg!(target_os = "macos") {
                    "⌘"
                } else {
                    "Ctrl"
                };
                match app.pointer.hover {
                    Hover::MarkdownLinkText(_) => {
                        let hint = t!("preview-markdown-link-hint", key = key);
                        (*rect, format!("{text}\n{hint}"))
                    }
                    _ => (*rect, text),
                }
            }
            // 日志列表中的作者头像：提示作者名和邮箱。
            Hover::Log(crate::app::git::log::LogTarget::Avatar(i)) => {
                let hit = app
                    .hits
                    .log
                    .iter()
                    .find(|(_, t)| *t == crate::app::git::log::LogTarget::Avatar(i));
                let commit = app
                    .editor
                    .active()
                    .and_then(|d| d.log.as_deref())
                    .and_then(|v| v.commits.get(i));
                match (hit, commit) {
                    (Some(&(rect, _)), Some(c)) => (rect, format!("{} <{}>", c.author, c.email)),
                    _ => return,
                }
            }
            // 提交图中线段的目标：点击后跳转到的提交。
            Hover::Log(crate::app::git::log::LogTarget::Line(row, line)) => {
                let target = crate::app::git::log::LogTarget::Line(row, line);
                if !app.hits.log.iter().any(|(_, t)| *t == target) {
                    return;
                }
                // 线段的命中框很小，提示放在鼠标下方，不按命中框居中。
                let (mx, my) = app.mouse();
                let rect = Rect::new(mx, my + 6.0, 0.0, 0.0);
                let commit = app.log_line_target(row, line).and_then(|j| {
                    app.editor
                        .active()
                        .and_then(|d| d.log.as_deref())
                        .and_then(|v| v.commits.get(j))
                });
                let label = match commit {
                    Some(c) => t!(
                        "workspace-tip-log-go-to",
                        commit = crate::app::git::log::short(&c.oid),
                        subject = c.subject.as_str()
                    ),
                    None => t!("workspace-tip-log-target-not-loaded").to_owned(),
                };
                (rect, label)
            }
            // 日志分支面板中截短了的分支、标签和目录名：提示全名。
            Hover::Log(crate::app::git::log::LogTarget::BranchRow(i))
                if app.git.log.long_branches.contains(&i) =>
            {
                use crate::app::git::log::{BranchRow, LogTarget};
                let hit = app
                    .hits
                    .log
                    .iter()
                    .find(|(_, t)| *t == LogTarget::BranchRow(i));
                let row = app.log_branch_rows().into_iter().nth(i);
                let name = match row {
                    Some(BranchRow::Branch { index, .. }) => {
                        app.git.branches.get(index).map(|b| b.name.clone())
                    }
                    Some(BranchRow::Tag(index)) => app.git.tags.get(index).cloned(),
                    Some(BranchRow::Folder { path, .. }) => Some(path),
                    _ => None,
                };
                match (hit, name) {
                    (Some(&(rect, _)), Some(name)) => (rect, name),
                    _ => return,
                }
            }
            // 日志详情中目录被截短或文件被改名的条目：显示完整路径。
            Hover::Log(crate::app::git::log::LogTarget::File(i))
                if app.git.log.long_files.contains(&i) =>
            {
                let hit = app
                    .hits
                    .log
                    .iter()
                    .find(|(_, t)| *t == crate::app::git::log::LogTarget::File(i));
                match (hit, app.log_file_tip(i)) {
                    (Some(&(rect, _)), Some(tip)) => (rect, tip),
                    _ => return,
                }
            }
            // 日志列表中延伸到右端仍放不下的提交说明：提示整句。
            Hover::Log(crate::app::git::log::LogTarget::Row(i))
                if app.git.log.clipped == Some(i) =>
            {
                let hit = app
                    .hits
                    .log
                    .iter()
                    .find(|(_, t)| *t == crate::app::git::log::LogTarget::Row(i));
                let commit = app
                    .editor
                    .active()
                    .and_then(|d| d.log.as_deref())
                    .and_then(|v| v.commits.get(i));
                match (hit, commit) {
                    (Some(&(rect, _)), Some(c)) => (rect, c.subject.clone()),
                    _ => return,
                }
            }
            _ => return,
        },
    };
    use crate::app::animation::TipPlace;
    use crate::app::git::log::LogTarget;
    let place = match app.pointer.hover {
        Hover::RulerChange(_) => TipPlace::Beside,
        // 列表的一行：提示从开始悬停时鼠标的位置开始。
        Hover::Log(LogTarget::Row(_) | LogTarget::BranchRow(_) | LogTarget::File(_)) => {
            TipPlace::Pointer
        }
        _ => TipPlace::Below,
    };
    let button = match place {
        TipPlace::Pointer => Rect::new(app.overlays.tooltip.pointer_x, button.y, 0.0, button.h),
        _ => button,
    };
    let tip = (button, label.as_str(), place);
    draw_tip(app, frame, tip, app.tooltip_opacity());
    *app.overlays.tooltip.last.borrow_mut() = Some((button, label, place));
}

/// 画一条提示：对着 `button`，文字 `label`，位置见 [`crate::app::animation::TipPlace`]。整条
/// 按 `opacity` 变淡，用于淡入淡出。
fn draw_tip(
    app: &App,
    frame: &mut Frame<'_>,
    tip: (Rect, &str, crate::app::animation::TipPlace),
    opacity: f32,
) {
    let (button, label, place) = tip;
    frame.with_opacity(opacity, |frame| {
        draw_tip_card(app, frame, button, label, place)
    });
}

fn draw_tip_card(
    app: &App,
    frame: &mut Frame<'_>,
    button: Rect,
    label: &str,
    place: crate::app::animation::TipPlace,
) {
    use crate::app::animation::TipPlace;
    let p = app.palette;
    // 提示分「名称   快捷键」两栏，快捷键以淡色显示在右侧，两栏间距至少 16 逻辑像素。含 `\n` 时
    // 第一段为正文，其余各段为淡色说明（如标签的文件名和所在目录）。
    let (name, key) = match label.split_once(crate::layout::TIP_KEY) {
        Some((name, key)) => (name, Some(key)),
        None => (label, None),
    };
    let size = app.fonts.ui_small;
    let key_w = key.map_or(0.0, |k| 16.0 + frame.measure(FontKind::Ui, size, k));
    // 提示宽度不超过 `TOOLTIP_MAX_WIDTH`，也不超出窗口（左右各留 4 逻辑像素边距）；
    // 文字区再减去左右各 10 逻辑像素的内边距。
    let max_width = TOOLTIP_MAX_WIDTH.min(app.layout.window.w - 8.0);
    let room = max_width - 20.0;
    let lines = tip_lines(name, room, key_w, |piece| {
        frame.measure(FontKind::Ui, size, piece)
    });
    let name_w = lines
        .iter()
        .map(|(line, _)| frame.measure(FontKind::Ui, size, line))
        .fold(0.0, f32::max);
    let width = (name_w + key_w + 20.0).min(max_width);
    // 单行高 26 逻辑像素，每多一行增加 18。
    const LINE: f32 = 18.0;
    let height = 8.0 + LINE * lines.len() as f32;
    let rect = if place == TipPlace::Beside {
        // 概览标尺紧靠窗口右边缘：提示放在标尺左侧、与鼠标同高，不遮挡鼠标所指的标记。
        Rect::new(
            button.x - 8.0 - width,
            button.y + button.h / 2.0 - height / 2.0,
            width,
            height,
        )
    } else {
        let below = button.bottom() + 6.0;
        let y = if below + height > app.layout.window.bottom() {
            button.y - 6.0 - height
        } else {
            below
        };
        // 对着一行时文字从鼠标处开始，对着按钮时居中。
        let x = match place {
            TipPlace::Pointer => button.x - 10.0,
            _ => button.x + (button.w - width) / 2.0,
        };
        let right = (app.layout.window.right() - width - 4.0).max(4.0);
        Rect::new(x.clamp(4.0, right), y, width, height)
    };
    draw_card(frame, p, rect, RADIUS_ROW, p.popup_bg);
    for (i, &(line, body)) in lines.iter().enumerate() {
        let row = Rect::new(rect.x, rect.y + 4.0 + LINE * i as f32, rect.w, LINE);
        let color = if body { p.text } else { p.text_dim };
        ui_text_small(frame, app, rect.x + 10.0, row, line, color, rect);
    }
    if let Some(key) = key {
        let w = frame.measure(FontKind::Ui, size, key);
        let row = Rect::new(rect.x, rect.y + 4.0, rect.w, LINE);
        ui_text_small(
            frame,
            app,
            rect.right() - 10.0 - w,
            row,
            key,
            p.text_dim,
            rect,
        );
    }
}

/// 把提示正文 `name` 按宽度 `room`（逻辑像素）折行，返回各行及其是否属于正文段。
///
/// `name` 以 `\n` 分段，第一段为正文，其余为说明；正文折出的各行都属于正文段。快捷键只占
/// 第一行，因此第一段的宽度再减去 `key_w`。`measure` 返回一段文字的宽度（逻辑像素）。
pub(super) fn tip_lines(
    name: &str,
    room: f32,
    key_w: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<(&str, bool)> {
    let mut lines = Vec::new();
    for (i, part) in name.split('\n').enumerate() {
        let width = (room - if i == 0 { key_w } else { 0.0 }).max(1.0);
        let wrapped = crate::layout::wrap_text(part, width, &mut measure);
        lines.extend(wrapped.into_iter().map(|line| (line, i == 0)));
    }
    lines
}
/// 返回面板内按钮的悬停提示（按钮矩形和提示文本）；没有提示时返回 `None`。
pub(in crate::view) fn tooltip(app: &App) -> Option<(Rect, String)> {
    use crate::app::{CommitTarget, DiffTarget};
    use crate::keymap::Action;
    use crate::layout::with_key;
    match app.pointer.hover {
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::New) => {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            Some((rect, t!("terminal-new").to_owned()))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Tab(i))
            if app.terminal_tab_list().is_some() =>
        {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            Some((rect, app.terminal.tabs.get(i)?.label().to_owned()))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::ShellMenu) => {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            Some((rect, t!("terminal-shell-select").to_owned()))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Find(part)) => {
            use crate::app::terminal::FindPart;
            let label = match part {
                FindPart::Bar => return None,
                FindPart::Previous => t!("cmd-find-previous"),
                FindPart::Next => t!("cmd-find-next"),
                FindPart::Close => t!("workspace-find-close"),
                FindPart::Case => t!("workspace-find-match-case"),
                FindPart::Word => t!("workspace-find-whole-word"),
                FindPart::Regex => t!("workspace-find-use-regex"),
            };
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            Some((rect, label.to_owned()))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Maximize) => {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            let label = if app.terminal.maximized {
                t!("terminal-restore-panel")
            } else {
                t!("terminal-maximize-panel")
            };
            Some((rect, with_key(label, Some(Action::MaximizeTerminal))))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Position) => {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            let label = if app.terminal_right() {
                t!("terminal-dock-bottom")
            } else {
                t!("terminal-dock-right")
            };
            Some((rect, label.to_owned()))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Hide) => {
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            Some((
                rect,
                with_key(t!("terminal-hide-panel"), Some(Action::ToggleTerminalPanel)),
            ))
        }
        Hover::TerminalHeader(hit @ crate::app::terminal::HeaderHit::Run(button)) => {
            use crate::app::terminal::run::RunButton;
            let rect = app.hits.terminal.iter().find(|(_, h)| *h == hit)?.0;
            let label = match button {
                RunButton::Menu => t!("terminal-run-select").to_owned(),
                RunButton::Start if app.run.list.is_empty() => t!("terminal-run-add").to_owned(),
                RunButton::Start if app.run_config_running() => {
                    with_key(t!("terminal-run-again"), Some(Action::RunConfiguration))
                }
                RunButton::Start => {
                    with_key(t!("cmd-run-configuration"), Some(Action::RunConfiguration))
                }
                RunButton::Stop => with_key(
                    t!("cmd-stop-configuration"),
                    Some(Action::StopConfiguration),
                ),
            };
            Some((rect, label))
        }
        Hover::Status(item @ crate::app::StatusItem::Terminal) => {
            let rect = app.hits.status.iter().find(|(_, i)| *i == item)?.0;
            let label = if app.terminal.open {
                t!("terminal-hide-panel")
            } else {
                t!("terminal-show-panel")
            };
            Some((rect, with_key(label, Some(Action::ToggleTerminalPanel))))
        }
        Hover::Status(item @ crate::app::StatusItem::Remote) => {
            let rect = app.hits.status.iter().find(|(_, i)| *i == item)?.0;
            // 已打开远端目录时说明连着哪台主机。
            let host = (app.editor.root())
                .and_then(winc_backend::locate)
                .map(|location| location.target);
            let label = match host {
                Some(host) if !winc_backend::remote::is_connected(&host) => {
                    t!("workspace-remote-status-lost", host = host.as_str())
                }
                Some(host) if winc_backend::remote::is_stalled(&host) => {
                    t!("workspace-remote-status-stalled", host = host.as_str())
                }
                Some(host) => t!("workspace-remote-status-connected", host = host.as_str()),
                None => t!("workspace-start-remote").to_owned(),
            };
            Some((rect, with_key(&label, Some(Action::RemoteConnect))))
        }
        Hover::Status(item @ crate::app::StatusItem::ReadOnly) => {
            let rect = app.hits.status.iter().find(|(_, i)| *i == item)?.0;
            let label = with_key(t!("cmd-toggle-read-only"), Some(Action::ToggleReadOnly));
            Some((rect, label))
        }
        Hover::Status(item @ crate::app::StatusItem::Update) => {
            let rect = app.hits.status.iter().find(|(_, i)| *i == item)?.0;
            Some((rect, crate::view::status::update_tooltip(app)?))
        }
        Hover::Status(item @ crate::app::StatusItem::Settings) => {
            let rect = app.hits.status.iter().find(|(_, i)| *i == item)?.0;
            Some((rect, with_key(t!("cmd-settings"), Some(Action::Settings))))
        }
        // 分支选择器的拉取、推送、获取：写明会做什么（和谁差几个提交、推到哪儿）；不能用时写原因。
        Hover::PickerPart(
            part @ (crate::app::PickerPart::Pull
            | crate::app::PickerPart::Push
            | crate::app::PickerPart::Fetch),
        ) => {
            let rect = app.hits.picker_parts.iter().find(|(_, p)| *p == part)?.0;
            let command = app.picker_action_command(part);
            let label = app.menu_label(command);
            let tip = if app.git.remotes.is_empty() {
                t!("git-push-no-remote").to_owned()
            } else {
                label.subtitle.unwrap_or(label.title)
            };
            Some((rect, tip))
        }
        // 提交按钮：无法提交时显示原因，可以提交时显示提交目标和快捷键；按住 ⌥ 时是提交并推送。
        Hover::Commit(CommitTarget::Commit) => {
            use winc_editor::git::{Head, Operation};
            let parts = app.commit_parts();
            let status = app.git.status.as_ref()?;
            if app.git.commit.busy {
                return None;
            }
            if let Some(reason) = app.commit_blocker(status) {
                return Some((parts.commit, reason));
            }
            let name = match (status.operation, &status.head) {
                (Some(Operation::Rebase), _) => t!("workspace-tip-continue-rebase").to_owned(),
                (Some(op), _) => t!("workspace-tip-finish-operation", operation = op.label()),
                (None, _) if app.git.commit.amend => t!("workspace-tip-amend").to_owned(),
                (None, Some(Head::Branch(b))) if app.commit_pushes() => {
                    t!("workspace-tip-commit-push-to", branch = b.as_str())
                }
                (None, Some(Head::Branch(b))) => t!("workspace-tip-commit-to", branch = b.as_str()),
                (None, _) => t!("workspace-tip-commit").to_owned(),
            };
            let key = match (cfg!(target_os = "macos"), app.commit_pushes()) {
                (true, true) => "⌥⌘↩",
                (true, false) => "⌘↩",
                (false, true) => "Ctrl+Alt+Enter",
                (false, false) => "Ctrl+Enter",
            };
            Some((
                parts.commit,
                format!("{name}{}{key}", crate::layout::TIP_KEY),
            ))
        }
        Hover::Commit(
            target @ (CommitTarget::OpenFile(_)
            | CommitTarget::RevertFile(_)
            | CommitTarget::GroupRevert(_)),
        ) => {
            let rect = app.hits.commit.iter().find(|(_, t)| *t == target)?.0;
            let label = match target {
                CommitTarget::OpenFile(_) => t!("workspace-tip-open-file"),
                CommitTarget::GroupRevert(_) => t!("menu-commit-roll-back"),
                _ => t!("workspace-tip-revert-change"),
            };
            Some((rect, label.to_owned()))
        }
        Hover::Commit(
            target @ (CommitTarget::StageFile(..)
            | CommitTarget::GroupStage(_)
            | CommitTarget::DirStage(..)),
        ) => {
            let rect = app.hits.commit.iter().find(|(_, t)| *t == target)?.0;
            let unstage = matches!(
                target,
                CommitTarget::StageFile(_, false)
                    | CommitTarget::DirStage(_, false)
                    | CommitTarget::GroupStage(crate::app::CommitGroup::Staged)
            );
            let label = if unstage {
                t!("menu-git-unstage")
            } else {
                t!("menu-git-stage")
            };
            Some((rect, label.to_owned()))
        }
        // 同步控件上没有文字，要做的事写在这里。
        Hover::Commit(target @ CommitTarget::Sync) => {
            let rect = app.hits.commit.iter().find(|(_, t)| *t == target)?.0;
            let upstream = (app.git.branches.iter())
                .find(|b| b.current)
                .and_then(|b| b.upstream.as_deref())
                .unwrap_or_default();
            let label = match (app.git.branch.running.as_deref(), app.git_sync_state()) {
                (Some(task), _) => t!("workspace-status-branch-running", task = task),
                (None, SyncState::Publish) => t!("git-tip-publish").to_owned(),
                (None, SyncState::InSync) => t!("git-tip-in-sync", upstream = upstream),
                (None, SyncState::Push(count)) => {
                    t!("git-tip-push", count = count, upstream = upstream)
                }
                (None, SyncState::Pull(count)) => {
                    t!("git-tip-pull", count = count, upstream = upstream)
                }
                (None, SyncState::Both { ahead, behind }) => t!(
                    "git-tip-both",
                    ahead = ahead,
                    behind = behind,
                    upstream = upstream
                ),
                (None, SyncState::NoRemote | SyncState::Hidden) => t!("git-tip-fetch").to_owned(),
            };
            Some((rect, label))
        }
        Hover::MarkdownToc(target @ crate::app::preview::markdown::TocTarget::Pin) => {
            let rect = app.hits.markdown_toc.iter().find(|(_, t)| *t == target)?.0;
            let label = match app.state.markdown_toc_pinned {
                true => t!("preview-markdown-toc-unpin"),
                false => t!("preview-markdown-toc-pin"),
            };
            Some((rect, label.to_owned()))
        }
        Hover::Outline(target @ OutlineTarget::FoldAll) => {
            let rect = app.hits.outline.iter().find(|(_, t)| *t == target)?.0;
            let label = if app.outline_any_expanded() {
                t!("workspace-outline-tool-collapse-all")
            } else {
                t!("workspace-outline-tool-expand-all")
            };
            Some((rect, label.to_owned()))
        }
        // 工作树一组收起时各行不可见：其他工作树有更改时在标题上提示。
        Hover::Commit(target @ CommitTarget::Worktrees) if !app.git.commit.worktrees.open => {
            let rect = app.hits.commit.iter().find(|(_, t)| *t == target)?.0;
            let count = app.git_other_changes();
            (count > 0).then(|| (rect, t!("git-tip-other-worktrees", count = count)))
        }
        Hover::Commit(target @ CommitTarget::WorktreeNew) => {
            let rect = app.hits.commit.iter().find(|(_, t)| *t == target)?.0;
            Some((rect, t!("git-branch-picker-new-worktree").to_owned()))
        }
        Hover::Commit(CommitTarget::Location) => None,
        Hover::Commit(CommitTarget::CommitMenu) => Some((
            app.commit_parts().commit_menu,
            t!("workspace-tip-commit-menu").to_owned(),
        )),
        Hover::Commit(target) => {
            let rect = app
                .commit_parts()
                .tools
                .iter()
                .find(|(_, t)| *t == target)?
                .0;
            if target == CommitTarget::Log {
                return Some((
                    rect,
                    with_key(t!("workspace-tip-git-log"), Some(Action::GitShowLog)),
                ));
            }
            if target == CommitTarget::Refresh {
                return Some((rect, t!("workspace-tip-git-refresh").to_owned()));
            }
            Some((rect, t!("workspace-tip-more-actions").to_owned()))
        }
        Hover::Merge(target) => {
            use crate::app::git::merge::MergeTarget;
            let rect = app.hits.merge.iter().find(|(_, t)| *t == target)?.0;
            let label = match target {
                MergeTarget::Previous => with_key(
                    t!("workspace-tip-previous-conflict"),
                    Some(Action::PreviousDifference),
                ),
                _ => with_key(
                    t!("workspace-tip-next-conflict"),
                    Some(Action::NextDifference),
                ),
            };
            Some((rect, label))
        }
        // 附属工作树：写明是哪个仓库的工作树、在哪里（项目名写的是工作树自己的目录名）。
        Hover::ProjectName => {
            let main = app.git.main_worktree()?;
            let root = app.git.root.as_deref()?;
            let name = crate::app::git::worktree::worktree_name(main);
            let path = winc_editor::display_path(root, None);
            let tip = t!("workspace-tip-worktree-of", name = name, path = path);
            Some((app.hits.project, tip))
        }
        Hover::RulerChange(hunk) => app.ruler_tip(hunk),
        Hover::Blame(line) => app.blame_tip(line),
        Hover::LatexStatus => Some((app.hits.latex_status, app.latex_status_tip()?)),
        Hover::LatexProblem(line) => app.latex_problem_tip(line),
        Hover::Change(target) => {
            use crate::app::ChangeTarget;
            let rect = app.hits.changes.iter().find(|(_, t)| *t == target)?.0;
            let label = match target {
                ChangeTarget::Previous => with_key(
                    t!("workspace-tip-previous-change"),
                    Some(Action::PreviousDifference),
                ),
                ChangeTarget::Next => with_key(
                    t!("workspace-tip-next-change"),
                    Some(Action::NextDifference),
                ),
                ChangeTarget::Revert => with_key(
                    t!("workspace-tip-revert-change"),
                    Some(Action::GitRevertChange),
                ),
                ChangeTarget::ShowDiff => {
                    with_key(t!("workspace-tip-open-diff"), Some(Action::GitShowChanges))
                }
                ChangeTarget::Close => t!("workspace-tip-close-esc").to_owned(),
            };
            Some((rect, label))
        }
        Hover::Diff(target) => {
            let rect = app.hits.diff.iter().find(|(_, t)| *t == target)?.0;
            let label = match target {
                DiffTarget::Previous => with_key(
                    t!("workspace-tip-previous-change"),
                    Some(Action::PreviousDifference),
                ),
                DiffTarget::Next => with_key(
                    t!("workspace-tip-next-change"),
                    Some(Action::NextDifference),
                ),
                DiffTarget::PreviousFile => with_key(
                    t!("workspace-tip-previous-file"),
                    Some(Action::PreviousDiffFile),
                ),
                DiffTarget::NextFile => {
                    with_key(t!("workspace-tip-next-file"), Some(Action::NextDiffFile))
                }
                DiffTarget::OpenFile => t!("workspace-tip-open-file").to_owned(),
                DiffTarget::Blame(side, line) => app.diff_blame_tip(side, line)?,
                DiffTarget::Layout if app.editor.diff_unified => {
                    t!("workspace-tip-diff-side-by-side").to_owned()
                }
                DiffTarget::Layout => t!("workspace-tip-diff-inline").to_owned(),
                DiffTarget::FilesToggle if app.log_files_open() => {
                    t!("git-log-diff-hide-files").to_owned()
                }
                DiffTarget::FilesToggle => t!("git-log-diff-show-files").to_owned(),
                DiffTarget::FilesLayout if app.diff_files_flat() => {
                    t!("workspace-tip-show-as-tree").to_owned()
                }
                DiffTarget::FilesLayout => t!("workspace-tip-show-as-list").to_owned(),
                // 文件栏里截短或改过名的文件：完整路径；其余文件不提示。
                DiffTarget::File(i) => app.log_diff_file_tip(i)?,
                DiffTarget::Revert(_) => with_key(
                    t!("workspace-tip-revert-to-left"),
                    Some(Action::GitRevertChange),
                ),
                DiffTarget::Include(_) => t!("workspace-tip-include-change").to_owned(),
                DiffTarget::IncludeLine(..) => t!("workspace-tip-include-line").to_owned(),
                _ if app.editor.active()?.diff.as_deref()?.collapses() => {
                    t!("workspace-tip-expand-unchanged").to_owned()
                }
                _ => t!("workspace-tip-collapse-unchanged").to_owned(),
            };
            Some((rect, label))
        }
        // 标签提示显示完整名称和所在目录，便于确认被截短的文件名和历史差异的版本；
        // 关闭按钮显示快捷键。按住或拖动标签时不显示提示。
        Hover::Tab(index, close) => {
            if app.tabs.drag.is_some() {
                return None;
            }
            let hit = app.hits.tabs.iter().find(|h| h.index == index)?;
            if close {
                return Some((
                    hit.close,
                    with_key(t!("workspace-tip-close-tab"), Some(Action::CloseTab)),
                ));
            }
            let doc = app.editor.documents.get(index)?;
            let title = doc.name();
            let path = (doc.path.as_deref()).or_else(|| {
                doc.diff
                    .as_deref()
                    .and_then(winc_editor::diff_view::DiffView::file_path)
            });
            let label = match path.and_then(std::path::Path::parent) {
                Some(dir) => format!("{title}\n{}", winc_editor::display_path(dir, None)),
                // 没有对应文件的标签（未命名文件、日志等）只在名称被截短时提示完整名称。
                None if hit.elided => title.into_owned(),
                None => return None,
            };
            Some((hit.tab, label))
        }
        Hover::TabList => Some((app.tabs.list?, t!("workspace-tip-all-tabs").to_owned())),
        Hover::Panel(PanelButton::Tool(tool)) => {
            let parts = app.panel_parts();
            let rect = parts.tools.iter().find(|(_, t)| *t == tool)?.0;
            let label = match tool {
                PanelTool::Refresh => t!("workspace-search-tool-refresh"),
                PanelTool::Clear => t!("workspace-search-tool-clear"),
                PanelTool::CollapseAll if app.panels.global.all_collapsed() => {
                    t!("workspace-search-tool-expand-all")
                }
                PanelTool::CollapseAll => t!("workspace-search-tool-collapse-all"),
            };
            Some((rect, label.to_owned()))
        }
        Hover::Panel(
            button @ (PanelButton::Expand
            | PanelButton::Case
            | PanelButton::Word
            | PanelButton::Regex
            | PanelButton::ReplaceAll),
        ) => {
            // 提示文本沿用查找条上对应按钮的文本。
            let parts = app.panel_parts();
            let (rect, same) = match button {
                PanelButton::Expand => (parts.expand, SearchButton::Expand),
                PanelButton::Case => (parts.case_toggle, SearchButton::Case),
                PanelButton::Word => (parts.word_toggle, SearchButton::Word),
                PanelButton::Regex => (parts.regex_toggle, SearchButton::Regex),
                _ => (parts.replace_all, SearchButton::ReplaceAll),
            };
            Some((rect, same.label()))
        }
        Hover::Panel(PanelButton::Ignored) => Some((
            app.panel_parts().ignored_toggle,
            t!("workspace-search-include-ignored").to_owned(),
        )),
        Hover::Panel(PanelButton::Details) => {
            let label = if app.panels.global.details_open {
                t!("workspace-search-hide-filters")
            } else {
                t!("workspace-search-filter-files")
            };
            Some((app.panel_parts().details, label.to_owned()))
        }
        Hover::RowAction(index, action) => {
            let (replace, dismiss) = app.result_actions(index);
            let is_file = matches!(
                app.panels.global.rows().get(index),
                Some(ResultRow::File(_))
            );
            match action {
                RowAction::Replace => {
                    let label = if is_file {
                        t!("workspace-search-row-replace-all")
                    } else {
                        t!("workspace-search-row-replace")
                    };
                    Some((replace?, label.to_owned()))
                }
                RowAction::Dismiss => {
                    Some((dismiss, t!("workspace-search-row-dismiss").to_owned()))
                }
            }
        }
        _ => None,
    }
}
