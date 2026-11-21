//! 左栏顶部的工作区标签条（多工作区模式）。
//!
//! 窗口里打开的每个项目在左栏顶部占一个标签：项目徽章和名字。当前打开的那个标签高亮，
//! 点别的标签就切过去；标签悬停时右端出现关闭按钮。末尾是「打开项目」与项目菜单两个按钮，
//! 标签放不下时在它们左边横向滚动。

use super::*;

/// 左栏顶部的工作区标签条。
#[derive(Debug, Default)]
pub(crate) struct WorkspaceBar {
    /// 各个工作区标签，按显示顺序排列。
    pub chips: Vec<WorkspaceChip>,
    /// 末尾「打开项目」按钮的位置。
    pub add: Rect,
    /// 末尾项目菜单按钮的位置。
    pub more: Rect,
    /// 标签滚动区域：标签在其中滚动，末尾两个按钮不在其中。
    pub view: Rect,
    /// 横向滚动位置（放不下时）。
    pub scroll: f32,
}

/// 左栏顶部的一个工作区标签。
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkspaceChip {
    /// 标签在窗口中的位置。
    pub rect: Rect,
    /// 标签里关闭按钮的位置。
    pub close: Rect,
    /// 工作区在 `workspaces.slots` 中的下标。
    pub index: usize,
}

impl WorkspaceBar {
    /// 返回工作区 `index` 的标签位置；没有时返回 `None`。
    pub fn chip(&self, index: usize) -> Option<Rect> {
        self.chips.iter().find(|c| c.index == index).map(|c| c.rect)
    }
}
