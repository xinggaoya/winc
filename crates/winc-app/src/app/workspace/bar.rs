//! 左栏顶部的工作区标签条（多工作区模式）。
//!
//! 窗口里打开的每个项目在左栏顶部占一个标签：项目徽章和名字。当前打开的那个标签高亮，
//! 点别的标签就切过去；标签悬停时右端出现关闭按钮。末尾是「打开项目」与项目菜单两个按钮，
//! 标签放不下时在它们左边横向滚动。

use super::*;

/// 判定为拖动的移动距离（逻辑像素）：按下后移动不足此距离时仍视为单击。
const THRESHOLD: f32 = 4.0;

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
    /// 正在拖动的工作区标签。
    pub drag: Option<WorkspaceDrag>,
}

/// 被按住的工作区标签的拖动状态。
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceDrag {
    /// 被拖动的工作区在 `workspaces.slots` 中的下标。
    pub index: usize,
    /// 按下点到标签左边的距离（逻辑像素）。
    pub grab: f32,
    /// 按下时鼠标的位置（逻辑像素）。
    pub start: (f32, f32),
    /// 移动是否已超过阈值，即是否已算作拖动。
    pub moving: bool,
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

impl App {
    /// 按下工作区标签 `index`：记下拖动的起点。
    pub(in crate::app) fn press_workspace(&mut self, index: usize, (x, y): (f32, f32)) {
        let Some(chip) = self.workspace_bar.chips.iter().find(|c| c.index == index) else {
            return;
        };
        let grab = x - chip.rect.x;
        self.workspace_bar.drag = Some(WorkspaceDrag {
            index,
            grab,
            start: (x, y),
            moving: false,
        });
    }

    /// 拖动工作区标签：标签越过相邻标签的中线时与它换位，顺序实时调整。
    pub(in crate::app) fn drag_workspace(&mut self, (x, y): (f32, f32)) {
        let Some(drag) = self.workspace_bar.drag else {
            return;
        };
        let (dx, dy) = (x - drag.start.0, y - drag.start.1);
        if !drag.moving && dx.abs() < THRESHOLD && dy.abs() < THRESHOLD {
            return;
        }
        self.workspace_bar.drag = Some(WorkspaceDrag {
            moving: true,
            ..drag
        });
        let at = self.workspace_drop_index(x);
        // 换位后被拖的工作区换了位置：记下它的新下标，否则下一次移动会动到别的工作区。
        let index = self.move_workspace(drag.index, at);
        self.workspace_bar.drag = Some(WorkspaceDrag { index, ..drag });
        self.request_redraw();
    }

    /// 松开：结束拖动。
    pub(in crate::app) fn release_workspace(&mut self) {
        if self.workspace_bar.drag.take().is_some() {
            self.request_redraw();
        }
    }

    /// 标签条整体平移了 `delta`（逻辑像素）时，把拖动中的抓取点跟着平移：抓取点是相对标签
    /// 左边的距离，标签平移后它也要跟着动，拖动中的标签才留在鼠标下。
    pub(in crate::app) fn shift_workspace_grab(&mut self, delta: f32) {
        if let Some(drag) = self.workspace_bar.drag {
            self.workspace_bar.drag = Some(WorkspaceDrag {
                grab: drag.grab + delta,
                ..drag
            });
        }
    }

    /// 拖动中的工作区标签该落在哪个位置：数出中心在鼠标左侧的标签个数（去掉它自己）。
    fn workspace_drop_index(&self, x: f32) -> usize {
        let Some(drag) = self.workspace_bar.drag else {
            return 0;
        };
        let mut at = 0;
        for chip in &self.workspace_bar.chips {
            if chip.index == drag.index {
                continue;
            }
            if x > chip.rect.x + chip.rect.w / 2.0 {
                at += 1;
            }
        }
        at
    }
}
