//! 切换工作区：把窗口里的现场和各个工作区的现场对调。
//!
//! 一个窗口里可以同时打开多个项目（工作区）。切换时，当前工作区的现场（标签、文件树、
//! Git 状态、终端）收进它自己的格子，目标工作区的现场取出来放到窗口里。终端是每个工作区
//! 一份，切过去时它仍在运行。

use super::*;

/// 把第 `index` 号工作区的现场和窗口里的对调。
fn swap_slot(app: &mut App, index: usize) {
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
    /// 当前工作区跟着走：被移的正是当前工作区时它落到新位置上，其余时候下标相应前移或后移。
    pub(in crate::app) fn move_workspace(&mut self, from: usize, to: usize) {
        let count = self.workspaces.slots.len();
        if from >= count || to >= count || from == to {
            return;
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
    }

    /// 关闭第 `index` 号工作区；关掉的是当前工作区时切到相邻的一个。
    pub(in crate::app) fn close_workspace(&mut self, index: usize) {
        if index >= self.workspaces.slots.len() {
            return;
        }
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
                return;
            }
            let next = index.min(self.workspaces.slots.len() - 1);
            self.workspaces.active = next;
            self.take_workspace(next);
            self.after_workspace_switch();
            return;
        }
        self.workspaces.slots.remove(index);
        if index < active {
            self.workspaces.active -= 1;
        }
        self.request_redraw();
    }
}
