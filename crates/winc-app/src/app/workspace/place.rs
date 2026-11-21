//! 顶栏里工作区切换条的位置：每个工作区一个按钮，末尾是「打开项目」按钮。

use super::*;

/// 工作区按钮里除文字以外的部分（图标和两侧的留白，逻辑像素）。
const CHIP_CHROME: f32 = 10.0 + 14.0 + 6.0 + 10.0;
/// 工作区按钮之间的间距（逻辑像素）。
const CHIP_GAP: f32 = 4.0;
/// 「打开项目」按钮的宽度（逻辑像素）。
const ADD_W: f32 = 26.0;

impl App {
    /// 排好顶栏里工作区切换条的位置，并把文件标签栏让到它的右边。
    ///
    /// 一个工作区也没有时不占地方。
    pub(in crate::app) fn place_workspaces(&mut self) {
        let l = &mut self.layout;
        l.workspaces = Rect::default();
        self.workspace_bar.chips.clear();
        self.workspace_bar.add = Rect::default();

        let count = self.workspaces.slots.len();
        // 只有一个工作区时不占地方：顶栏和以前一样，标签栏从侧栏开关右边开始。
        if count < 2 {
            return;
        }
        let Some(renderer) = self.frame.renderer.as_ref() else {
            return;
        };
        let font = self.fonts.ui;
        // 切换条占着标签栏最左边的一段，文件标签往右让。
        let left = l.tabs.x;
        let right = l.tabs.right();
        let y = l.top_bar.y + (l.top_bar.h - BUTTON) / 2.0;

        let mut widths = Vec::with_capacity(count);
        for slot in &self.workspaces.slots {
            let text = renderer.measure(FontKind::Ui, font, &slot.name);
            widths.push((CHIP_CHROME + text).ceil().max(48.0));
        }
        let total: f32 = widths.iter().sum::<f32>() + CHIP_GAP * count as f32 + ADD_W;

        let mut x = left;
        for (index, width) in widths.iter().enumerate() {
            let rect = Rect::new(x, y, *width, BUTTON);
            self.workspace_bar.chips.push((rect, index));
            x += width + CHIP_GAP;
        }
        let add = Rect::new(x, y, ADD_W, BUTTON);
        self.workspace_bar.add = add;
        self.workspace_bar.view = Rect::new(left, y, (add.right() - left).max(0.0), BUTTON);

        // 放不下时整条向左滚动，当前工作区的按钮始终可见。
        let room = (right - left).max(0.0);
        if total > room {
            let active = self.workspaces.active;
            let chip = self.workspace_bar.chip(active);
            let want = chip.map_or(0.0, |c| c.right() - right);
            self.workspace_bar.scroll = want.clamp(0.0, total - room);
        } else {
            self.workspace_bar.scroll = 0.0;
        }
        let scroll = self.workspace_bar.scroll;
        for (rect, _) in &mut self.workspace_bar.chips {
            rect.x -= scroll;
        }
        self.workspace_bar.add.x -= scroll;

        // 文件标签栏让到切换条右边。
        let end = self.workspace_bar.add.right() + TAB_GAP;
        if end > l.tabs.x {
            let shrink = end - l.tabs.x;
            l.tabs.x = end;
            l.tabs.w = (l.tabs.w - shrink).max(0.0);
        }
    }
}
