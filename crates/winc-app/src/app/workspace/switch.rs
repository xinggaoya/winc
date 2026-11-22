//! 切换工作区：把窗口里的现场和各个工作区的现场对调。
//!
//! 一个窗口里可以同时打开多个项目（工作区）。切换时，当前工作区的现场（标签、文件树、
//! Git 状态、终端）收进它自己的格子，目标工作区的现场取出来放到窗口里。终端是每个工作区
//! 一份，切过去时它仍在运行。

use super::*;

/// 把第 `index` 号工作区的现场和窗口里的对调。
pub(in crate::app) fn swap_slot(app: &mut App, index: usize) {
    let App {
        editor,
        git,
        terminal,
        run,
        panels,
        workspaces,
        ..
    } = app;
    let Some(slot) = workspaces.slots.get_mut(index) else {
        return;
    };
    std::mem::swap(&mut slot.editor, editor);
    std::mem::swap(&mut slot.git, git);
    std::mem::swap(&mut slot.terminal, terminal);
    std::mem::swap(&mut slot.run, run);
    std::mem::swap(&mut slot.tree, &mut panels.tree);
    std::mem::swap(&mut slot.global, &mut panels.global);
    std::mem::swap(&mut slot.outline, &mut panels.outline);
    // 名字跟着现场一起收进格子：用户给这一组起了名字时，标签上显示的是新名字。
    if let Some(workspace) = slot.editor.workspace.as_ref() {
        slot.name = workspace.name.clone();
    }
}

impl App {
    /// 多工作区模式是否开着：配置里没写时是开着的。
    pub(in crate::app) fn multi_workspace(&self) -> bool {
        self.config.multi_workspace != Some(false)
    }

    /// 启动时打开的目录登记为窗口里的第一个工作区。
    pub(in crate::app) fn with_workspace(mut self) -> Self {
        let Some(root) = self.editor.root().map(Path::to_path_buf) else {
            return self;
        };
        let name = self
            .editor
            .workspace
            .as_ref()
            .map(|ws| ws.name.clone())
            .unwrap_or_default();
        // 格子是空的，窗口里的是现场：切过去时正好换上格子里的。
        self.workspaces.slots.push(WorkspaceSlot::new(root, name));
        self.workspaces.active = 0;
        self
    }

    /// 把当前打开的目录登记成一个工作区。
    ///
    /// 已经登记过的目录不再登记；单工作区模式下只留当前这一个（换目录就是换一个）。
    pub(in crate::app) fn register_workspace(&mut self) {
        let Some(root) = self.editor.root().map(Path::to_path_buf) else {
            return;
        };
        if !self.multi_workspace() {
            self.workspaces.slots.clear();
        }
        if self.workspaces.index_of(&root).is_some() {
            return;
        }
        let name = self
            .editor
            .workspace
            .as_ref()
            .map(|ws| ws.name.clone())
            .unwrap_or_default();
        self.workspaces.slots.push(WorkspaceSlot::new(root, name));
        self.workspaces.active = self.workspaces.slots.len() - 1;
    }

    /// 切换到第 `index` 号工作区（从 0 起）。
    ///
    /// 已经在该工作区时不做任何事。
    pub(in crate::app) fn switch_workspace(&mut self, index: usize) {
        if index == self.workspaces.active || index >= self.workspaces.slots.len() {
            return;
        }
        // 退出时恢复用的现场：记下当前工作区的标签和终端。
        self.remember_session();
        // 收好当前工作区的现场，取出目标工作区的现场。
        self.stash_workspace();
        self.take_workspace(index);
        self.workspaces.active = index;
        self.after_workspace_switch();
    }

    /// 切换工作区之后：把与当前目录有关的东西重新挂接（监视目录、Git、终端尺寸）。
    fn after_workspace_switch(&mut self) {
        self.reset_folder_views();
        self.focus = Focus::Editor;
        // 目录变了：监视程序和 Git 的仓库跟着换。
        self.watch_folder();
        self.update_window_title();
        self.relayout();
        self.sync_terminal();
        self.request_redraw();
    }

    /// 切到下一个（`forward` 为真）或上一个工作区。
    pub(in crate::app) fn cycle_workspace(&mut self, forward: bool) {
        let count = self.workspaces.slots.len();
        if count < 2 {
            return;
        }
        let active = self.workspaces.active;
        let next = if forward {
            (active + 1) % count
        } else {
            (active + count - 1) % count
        };
        self.switch_workspace(next);
    }

    /// 打开目录 `path`：在多工作区模式下，它成了窗口里的另一个工作区。
    ///
    /// 已经开着这个目录时切过去；否则新开一个工作区并切过去。
    pub(in crate::app) fn open_workspace(&mut self, path: &Path) {
        let target = winc_config::files::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(index) = self.workspaces.index_of(&target) {
            self.switch_workspace(index);
            return;
        }
        let name = winc_editor::display_name(&target).into_owned();
        // 新工作区沿用当前的查找内容：接着在新目录里找。搜索弹窗开着时也一并开着。
        let query = self.panels.global.query.text().to_owned();
        let searching = self.search_open();
        // 终端面板开着时也一并开着：面板是窗口的样子，与目录无关。
        let terminal_open = self.terminal.open;
        // 先把当前工作区的现场收进它自己的格子。
        self.stash_workspace();
        // 新开一个工作区：格子是空的，把它放到窗口里，再在里面打开目录。
        self.workspaces
            .slots
            .push(WorkspaceSlot::new(target.clone(), name));
        let index = self.workspaces.slots.len() - 1;
        self.take_workspace(index);
        self.workspaces.active = index;
        if !query.is_empty() {
            self.panels.global.query.set_text(&query);
        }
        if searching {
            self.panels.global.open = true;
        }
        self.open_folder_now(&target);
        if terminal_open {
            self.set_terminal_open(true);
            // 新工作区里没有终端：面板开着，里面空着不像话，开一个（辅助进程是共用的，
            // 不用重新握手）。
            self.ensure_terminal_tab();
        }
        // 新工作区里没有提交面板时，焦点回到文件树：否则键入的文字会进入看不见的输入框。
        if !self.panel_available(self.panels.sidebar.view) {
            self.set_sidebar_view(winc_config::Panel::Files);
        }
    }

    /// 把窗口里的现场收进当前工作区的格子。
    pub(in crate::app) fn stash_workspace(&mut self) {
        let active = self.workspaces.active;
        if active >= self.workspaces.slots.len() {
            return;
        }
        swap_slot(self, active);
    }

    /// 把第 `index` 号工作区的现场取出来放到窗口里。
    pub(in crate::app) fn take_workspace(&mut self, index: usize) {
        if index >= self.workspaces.slots.len() {
            return;
        }
        swap_slot(self, index);
    }

    /// 把第 `from` 号工作区移到第 `to` 号的位置；`to` 是把它移除之后的下标。
    ///
    /// 返回被移动的工作区新的下标。当前工作区跟着走：被移的正是当前工作区时它落到新位置上，
    /// 其余时候下标相应前移或后移。
    pub(in crate::app) fn move_workspace(&mut self, from: usize, to: usize) -> usize {
        let count = self.workspaces.slots.len();
        if from >= count || to >= count || from == to {
            return from;
        }
        let slot = self.workspaces.slots.remove(from);
        self.workspaces.slots.insert(to, slot);
        let active = self.workspaces.active;
        self.workspaces.active = if active == from {
            to
        } else if from < active && to >= active {
            active - 1
        } else if from > active && to < active {
            active + 1
        } else {
            active
        };
        self.request_redraw();
        to
    }

    /// 关闭第 `index` 号工作区；关掉的是当前工作区时切到相邻的一个。
    ///
    /// 关掉后在提示条上留一个「撤销」：重新打开这个项目，位置回到关闭前的地方。
    pub(in crate::app) fn close_workspace(&mut self, index: usize) {
        self.close_workspace_inner(index, true);
    }

    /// 同 [`App::close_workspace`]，不在提示条上留「撤销」。
    ///
    /// 断开远端连接时用：关掉工作区是断开连接的结果，不是用户想关掉这个项目；撤销也开不回来。
    pub(in crate::app) fn close_workspace_without_undo(&mut self, index: usize) {
        self.close_workspace_inner(index, false);
    }

    /// 关闭第 `index` 号工作区；`undoable` 为真时在提示条上留一个「撤销」。
    fn close_workspace_inner(&mut self, index: usize, undoable: bool) {
        if index >= self.workspaces.slots.len() {
            return;
        }
        let folder = self.workspaces.slots[index].folder.clone();
        let name = self.workspace_name(index);
        let active = self.workspaces.active;
        if index == active {
            // 当前工作区：先记下它的现场（退出时恢复），再关掉它的终端。
            self.remember_session();
            self.close_all_terminal_tabs();
            self.stash_workspace();
            self.workspaces.slots.remove(index);
            if self.workspaces.slots.is_empty() {
                self.workspaces.active = 0;
                self.close_folder_now();
                self.notify_closed_workspace(&folder, &name, index, undoable);
                return;
            }
            let next = index.min(self.workspaces.slots.len() - 1);
            self.workspaces.active = next;
            self.take_workspace(next);
            self.after_workspace_switch();
            self.notify_closed_workspace(&folder, &name, index, undoable);
            return;
        }
        self.workspaces.slots.remove(index);
        if index < active {
            self.workspaces.active -= 1;
        }
        self.notify_closed_workspace(&folder, &name, index, undoable);
        self.request_redraw();
    }

    /// 提示工作区 `folder` 已关闭，可以在提示条上撤销。
    fn notify_closed_workspace(&mut self, folder: &Path, name: &str, index: usize, undoable: bool) {
        if !undoable {
            return;
        }
        self.notify_with(
            NoticeKind::Info,
            winc_i18n::t!("workspace-notice-workspace-closed", name = name),
            NoticeAction::UndoCloseWorkspace {
                folder: folder.to_path_buf(),
                index,
            },
        );
    }

    /// 撤销关闭工作区：重新打开这个项目，它的位置回到关闭前的地方。
    pub(in crate::app) fn undo_close_workspace(&mut self, folder: PathBuf, index: usize) {
        self.open_workspace(&folder);
        let Some(current) = self.workspaces.index_of(&folder) else {
            return;
        };
        let to = index.min(self.workspaces.slots.len() - 1);
        self.move_workspace(current, to);
        self.request_redraw();
    }

    /// 窗口里打开的工作区：开了哪些项目、顺序如何、当时打开的是哪一个。
    pub(crate) fn workspace_set(&self) -> WorkspaceSet {
        WorkspaceSet {
            folders: self
                .workspaces
                .slots
                .iter()
                .map(|slot| slot.folder.clone())
                .collect(),
            active: self.workspaces.active,
        }
    }

    /// 按 `set` 恢复窗口里打开的各个工作区。
    ///
    /// 启动时窗口里已经打开了上次退出时当前的那个工作区（它的现场已经恢复），其余的依次打开：
    /// 每个目录都按上次的现场打开，标签、终端都还在。最后按记下的顺序排列，并切到当时打开的
    /// 那一个。单工作区模式下只开第一个。
    pub(crate) fn restore_workspaces(&mut self, set: &WorkspaceSet) {
        if set.folders.is_empty() {
            return;
        }
        let current = self.editor.root().map(Path::to_path_buf);
        let folders: Vec<&PathBuf> = if self.multi_workspace() {
            set.folders.iter().collect()
        } else {
            set.folders.iter().take(1).collect()
        };
        for folder in folders {
            // 当前打开的就是它时不用再开。
            if Some(folder) == current.as_ref() || !winc_backend::fs::is_dir(folder) {
                continue;
            }
            self.open_workspace(folder);
        }
        if self.multi_workspace() {
            self.order_workspaces(set);
        }
    }

    /// 在窗口里打开 `folders` 里的各个目录：第一个作为当前打开的目录，其余各成一个工作区。
    ///
    /// `active` 是当前工作区在 `folders` 中的下标；越界时开最后一个。
    pub(crate) fn open_workspaces(&mut self, folders: &[PathBuf], active: usize) {
        let Some(first) = folders.first() else {
            return;
        };
        self.open_folder_now(first);
        for folder in &folders[1..] {
            self.open_workspace(folder);
        }
        // 切到记下的那一个。
        let index = active.min(folders.len() - 1);
        let Some(folder) = folders.get(index) else {
            return;
        };
        if let Some(index) = self.workspaces.index_of(folder) {
            self.switch_workspace(index);
        }
    }

    /// 按 `set` 记下的顺序排列工作区，并切到当时打开的那一个。
    ///
    /// 记下顺序里没有的工作区排在最后，仍按原来的相对次序。
    fn order_workspaces(&mut self, set: &WorkspaceSet) {
        let previous = self.workspaces.active;
        let mut slots: Vec<(usize, WorkspaceSlot)> =
            self.workspaces.slots.drain(..).enumerate().collect();
        slots.sort_by_key(|(index, slot)| {
            (
                set.folders
                    .iter()
                    .position(|folder| *folder == slot.folder)
                    .unwrap_or(usize::MAX),
                *index,
            )
        });
        // 窗口里的现场属于原来那个工作区：它在排序后的位置就是当前工作区。
        let active = slots.iter().position(|(index, _)| *index == previous);
        self.workspaces.slots = slots.into_iter().map(|(_, slot)| slot).collect();
        if let Some(active) = active {
            self.workspaces.active = active;
        }
        // 切到记下的那一个。
        let Some(folder) = set.folders.get(set.active) else {
            return;
        };
        if let Some(index) = self.workspaces.index_of(folder) {
            self.switch_workspace(index);
        }
    }
}
