//! 顶栏里切换各个工作区的按钮（多工作区模式）。
//!
//! 窗口里打开的每个项目在顶栏左侧占一个按钮：项目图标和名字。当前打开的那个按钮高亮，
//! 点别的按钮就切过去。末尾还有一个「打开项目」按钮。

use super::*;

/// 顶栏里的工作区切换条。
#[derive(Debug, Default)]
pub(crate) struct WorkspaceBar {
    /// 各个工作区按钮的位置，参数是工作区的下标。
    pub chips: Vec<(Rect, usize)>,
    /// 末尾「打开项目」按钮的位置；没有工作时为空矩形。
    pub add: Rect,
    /// 整条的位置。
    pub view: Rect,
    /// 横向滚动位置（放不下时）。
    pub scroll: f32,
}

impl WorkspaceBar {
    /// 返回工作区 `index` 的按钮位置；没有时返回 `None`。
    pub fn chip(&self, index: usize) -> Option<Rect> {
        self.chips
            .iter()
            .find(|(_, i)| *i == index)
            .map(|(rect, _)| *rect)
    }
}
