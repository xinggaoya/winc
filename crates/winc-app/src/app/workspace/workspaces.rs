//! 多工作区模式：同一个窗口里同时打开多个项目，随时切换。
//!
//! 每个工作区（打开的项目）有自己的现场：打开的目录、标签、文件树、Git 状态和终端。切换
//! 工作区时把当前窗口里的这些状态收进当前工作区，再把要切换过去的那个工作区的状态取出来，
//! 界面照常按同一个窗口绘制。
//!
//! 终端是每个工作区一份，切过去时它仍在运行：辅助进程（见 [`crate::app::terminal`]）由窗口
//! 共用，各工作区的终端标签挂在同一个辅助进程上。

use super::*;

use std::collections::HashSet;

use winc_editor::{DocumentId, Editor};

/// 一个工作区的现场。
pub(crate) struct WorkspaceSlot {
    /// 打开的目录。
    pub folder: PathBuf,
    /// 目录的显示名。
    pub name: String,
    /// 标签上用的名字：目录名（不含位置标记），用户起名时是起过的名字。
    pub label: String,
    /// 目录所在的主机；本机上的目录没有。
    pub host: Option<winc_editor::host::Host>,
    /// 编辑器状态：打开的文档、标签、窗格、书签和折叠的块。
    pub editor: Editor,
    /// Git 模块的状态。
    pub git: git::GitState,
    /// 终端面板。
    pub terminal: terminal::TerminalPanel,
    /// 当前目录的运行配置。
    pub run: terminal::run::RunConfigs,
    /// 文件树的界面状态。
    pub tree: workspace::tree::TreeState,
    /// 全局搜索的状态。
    pub global: GlobalSearch,
    /// 大纲的状态。
    pub outline: OutlineState,
}

impl WorkspaceSlot {
    /// 打开目录 `folder` 的工作区：状态都是空的，标签由调用方恢复。
    ///
    /// `name` 是显示用的名字（带位置标记），`label` 是标签上用的名字（不含位置标记）。
    pub(in crate::app) fn new(folder: PathBuf, name: String, label: String) -> Self {
        let host = winc_editor::host::host(&folder);
        Self {
            folder,
            name,
            label,
            host,
            editor: Editor::default(),
            git: git::GitState::default(),
            terminal: terminal::TerminalPanel::default(),
            run: terminal::run::RunConfigs::default(),
            tree: workspace::tree::TreeState::default(),
            global: GlobalSearch::default(),
            outline: OutlineState::default(),
        }
    }
}

/// 窗口里打开的各个工作区。
#[derive(Default)]
pub(crate) struct Workspaces {
    /// 各个工作区，第一个是当前打开的。
    pub slots: Vec<WorkspaceSlot>,
    /// 当前工作区在 `slots` 中的下标。
    pub active: usize,
}

impl Workspaces {
    /// 打开了目录 `folder` 的工作区的下标；没有时返回 `None`。
    pub fn index_of(&self, folder: &Path) -> Option<usize> {
        self.slots.iter().position(|slot| slot.folder == folder)
    }
}

impl App {
    /// 各个工作区里打开着的文档：当前编辑器和每个工作区格子里的编辑器都算。
    ///
    /// 备份不能因为文档不在当前编辑器里就删掉：别的工��区里开着的标签同样是在的。
    pub(in crate::app) fn live_documents(&self) -> HashSet<DocumentId> {
        let mut ids: HashSet<DocumentId> = self.editor.documents.iter().map(|d| d.id).collect();
        for slot in &self.workspaces.slots {
            ids.extend(slot.editor.documents.iter().map(|d| d.id));
        }
        ids
    }

    /// 找出文档 `id` 并给出可变的引用：当前编辑器里没有时到各个工作区里找。
    ///
    /// 后台保存的结果送到文档所在的工作区：文件写好了，那个工作区的标签不该还显示未保存。
    pub(in crate::app) fn document_mut_anywhere(
        &mut self,
        id: DocumentId,
    ) -> Option<&mut winc_editor::Document> {
        if self.editor.document(id).is_some() {
            return self.editor.document_mut(id);
        }
        let slot = self
            .workspaces
            .slots
            .iter_mut()
            .find(|slot| slot.editor.document(id).is_some())?;
        slot.editor.document_mut(id)
    }

    /// 工作区 `index` 的显示名：目录的名字，或用户给这一组起的名字。
    ///
    /// 当前工作区用打开着的目录的名字（起了名时用起名的），其余用登记时记下的。
    pub(crate) fn workspace_name(&self, index: usize) -> String {
        if index == self.workspaces.active
            && let Some(workspace) = self.editor.workspace.as_ref()
        {
            return workspace.name.clone();
        }
        self.workspaces
            .slots
            .get(index)
            .map_or_else(String::new, |slot| slot.name.clone())
    }

    /// 工作区 `index` 在标签上要画的两段文字：项目名与所在的主机。
    ///
    /// 标签上地方小：项目名单独占一行（末尾截断），位置另用小号淡色文字标出。
    ///
    /// 当前工作区的名字取活动编辑器里的那个：它可能刚被改过（改名、移除附加目录）。位置跟着
    /// 目录走：格子里记着的那份就是它。
    pub(crate) fn workspace_label(&self, index: usize) -> WorkspaceLabel {
        let name = if index == self.workspaces.active {
            self.editor
                .workspace
                .as_ref()
                .map_or_else(String::new, |workspace| workspace.label().to_owned())
        } else {
            self.workspaces
                .slots
                .get(index)
                .map_or_else(String::new, |slot| slot.label.clone())
        };
        let host = self
            .workspaces
            .slots
            .get(index)
            .and_then(|slot| slot.host.clone());
        WorkspaceLabel { name, host }
    }

    /// 当前工作区的名字与位置记回它的格子：刚打开的工作区、用户起名后都要跟上。
    pub(in crate::app) fn sync_active_slot(&mut self) {
        let index = self.workspaces.active;
        let Some(workspace) = self.editor.workspace.as_ref() else {
            return;
        };
        let Some(slot) = self.workspaces.slots.get_mut(index) else {
            return;
        };
        slot.name = workspace.name.clone();
        slot.label = workspace.label().to_owned();
    }
}

/// 工作区标签上要画的两段文字。
pub(crate) struct WorkspaceLabel {
    /// 项目名：目录名，或用户给这一组起的名字。
    pub name: String,
    /// 所在的主机；本机上的目录没有。
    pub host: Option<winc_editor::host::Host>,
}
