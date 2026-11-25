//! 打开的目录与文件的测试：快速打开、全局搜索、侧栏与文件树、拖放、标签栏、现场的保存与
//! 恢复、目录监视。

use super::*;

pub(super) mod columns;
pub(super) mod folders;
pub(super) mod loading;
pub(super) mod multi_workspace;
pub(super) mod outline;
mod panes;
pub(super) mod picker;
mod places;
mod project;
pub(super) mod recent;
pub(super) mod search;
// 全局搜索弹窗与「在文件中查找的对话框」的测试：本仓库与上游各自新增了同名模块，两边都保留，
// 合在一个文件里。
pub(super) mod search_dialog;
// 搜索结果以树形显示的测试。
mod search_tree;
pub(super) mod selection;
pub(super) mod session;
pub(super) mod shots;
pub(super) mod tab_selection;
pub(super) mod tabs;
pub(super) mod tree;
mod tree_drag;
pub(super) mod watch;
pub(super) mod workspaces;
