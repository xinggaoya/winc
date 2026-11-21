//! 左栏顶部工作区标签条的位置：每个工作区一个标签，末尾是「打开项目」与项目菜单两个按钮。
//!
//! 两个按钮钉在左栏的右端，标签在它们左边滚动；放不下时标签向左滚动，当前工作区的标签始终可见。

use super::*;

/// 标签里除文字以外的部分（徽章、关闭按钮与两侧留白，逻辑像素）。
pub(crate) const CHIP_CHROME: f32 = 6.0 + 20.0 + 5.0 + 4.0 + 16.0 + 6.0;
/// 标签的最小宽度（逻辑像素）。
pub(crate) const CHIP_MIN: f32 = 72.0;
/// 项目名最多占的宽度（逻辑像素），超出部分截短。
pub(crate) const CHIP_NAME_MAX: f32 = 140.0;
/// 末尾两个按钮的宽度（逻辑像素）。
const END_W: f32 = 24.0;
/// 标签与末尾按钮之间的间距（逻辑像素）。
const END_GAP: f32 = 4.0;
/// 标签里关闭按钮的边长（逻辑像素）。
const CLOSE: f32 = 16.0;

impl App {
    /// 排好左栏顶部工作区标签条的位置。
    ///
    /// 没有打开目录时只有末尾两个按钮；标签放不下时整条向左滚动，当前工作区的标签始终可见。
    pub(in crate::app) fn place_workspaces(&mut self) {
        let header = self.layout.sidebar_header;
        let y = header.y + (header.h - TAB_HEIGHT) / 2.0;
        let right = header.right() - 8.0;
        let count = self.workspaces.slots.len();
        // 各工作区的显示名：当前工作区用打开着的目录的名字，其余用登记时记下的。
        let names: Vec<String> = (0..count).map(|i| self.workspace_name(i)).collect();

        let bar = &mut self.workspace_bar;
        bar.chips.clear();
        bar.add = Rect::default();
        bar.more = Rect::default();
        bar.view = Rect::default();

        // 末尾两个按钮：项目菜单在最右，「打开项目」在它左边。
        let more = Rect::new(right - END_W, y, END_W, TAB_HEIGHT);
        let add = Rect::new(more.x - END_GAP - END_W, y, END_W, TAB_HEIGHT);
        bar.more = more;
        bar.add = add;

        let count = self.workspaces.slots.len();
        if count == 0 {
            return;
        }
        let Some(renderer) = self.frame.renderer.as_ref() else {
            return;
        };
        let font = self.fonts.ui;
        // 标签从左栏左边 8 像素处开始，滚到「打开项目」按钮左边为止。
        let left = header.x + 8.0;
        let room = (add.x - END_GAP - left).max(0.0);

        // 每个标签的宽度：徽章与关闭按钮占固定宽度，名字按长度算，长的截短。
        let mut widths = Vec::with_capacity(count);
        for name in &names {
            let text = renderer
                .measure(FontKind::Ui, font, name)
                .min(CHIP_NAME_MAX);
            widths.push((CHIP_CHROME + text).ceil().max(CHIP_MIN));
        }
        let total: f32 = widths.iter().sum::<f32>() + TAB_GAP * (count as f32 - 1.0);

        let mut x = left;
        for (index, width) in widths.iter().enumerate() {
            let rect = Rect::new(x, y, *width, TAB_HEIGHT);
            let close = Rect::new(
                rect.right() - 6.0 - CLOSE,
                y + (TAB_HEIGHT - CLOSE) / 2.0,
                CLOSE,
                CLOSE,
            );
            bar.chips.push(WorkspaceChip { rect, close, index });
            x += width + TAB_GAP;
        }

        // 放不下时整条向左滚动，当前工作区的标签始终可见。
        let scroll = if total > room {
            let active = self.workspaces.active;
            let chip = bar.chip(active);
            let want = chip.map_or(0.0, |c| c.right() - (left + room));
            want.clamp(0.0, total - room)
        } else {
            0.0
        };
        bar.scroll = scroll;
        for chip in &mut bar.chips {
            chip.rect.x -= scroll;
            chip.close.x -= scroll;
        }
        bar.view = Rect::new(left, y, room, TAB_HEIGHT);
        // 标签条的位置也记在布局里（上游的同名区域在顶栏，本仓库把它搬到了左栏）。
        self.layout.workspaces = bar.view;
    }
}
