//! 设置中「面板」页的几何：左右两栏里各有哪些面板、面板的顺序，以及拖动时各行的位置。
//!
//! 页面分三组：「左栏」「右栏」和「隐藏的面板」。每组里逐行列出面板，行首是拖动把手，
//! 行尾是几个按钮（上移、下移、移到另一栏、隐藏）。最后一行是右栏的宽度。

use winc_config::Panel;
use winc_render::Rect;

use super::*;
use crate::layout::Column;

/// 面板布局页里的一行：一个面板和它的几个按钮。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelRow {
    /// 面板的种类。
    pub panel: Panel,
    /// 面板所在的栏；隐藏的面板为 `None`。
    pub column: Option<Column>,
    /// 整行的位置。
    pub row: Rect,
    /// 行首的拖动把手。
    pub handle: Rect,
    /// 上移按钮。
    pub up: Rect,
    /// 下移按钮。
    pub down: Rect,
    /// 移到另一栏的按钮。
    pub side: Rect,
    /// 隐藏按钮（隐藏的面板上是「显示」）。
    pub hide: Rect,
}

/// 面板布局页的几何。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelsLayout {
    /// 左栏里各面板的行，从上到下。
    pub left: Vec<PanelRow>,
    /// 右栏里各面板的行，从上到下。
    pub right: Vec<PanelRow>,
    /// 隐藏的面板（不在任何一栏里）。
    pub hidden: Vec<PanelRow>,
    /// 各组的标题：标题所在行的位置和标题文字。
    pub headings: Vec<(Rect, &'static str)>,
    /// 各组的底色区域。
    pub groups: Vec<Rect>,
    /// 组内行与行之间的细线。
    pub separators: Vec<Rect>,
    /// 「右栏宽度」一行。
    pub width: Rect,
    /// 「多工作区模式」一行。
    pub multi_workspace: Rect,
    /// 该行右端的开关。
    pub multi_toggle: Rect,
    /// 宽度行右端的减号和加号。
    pub width_down: Rect,
    pub width_up: Rect,
    /// 内容的总高度（像素），用于计算最大滚动量。
    pub content_h: f32,
    /// 页面正文的高度（像素）：设置标题下方可以滚动的区域。
    pub body_h: f32,
}

/// 行尾四个按钮的宽度（逻辑像素）。
const BTN: f32 = 26.0;
/// 按钮之间的间距（逻辑像素）。
const BTN_GAP: f32 = 4.0;

impl PanelsLayout {
    /// 计算「面板」页的几何。
    ///
    /// `body` 是设置标题下方的区域；`left`、`right` 是两栏里的面板，顺序从上到下；
    /// `hidden` 是没有放在任何一栏里的面板；`scroll` 是滚动量（像素）。
    pub fn compute(
        body: Rect,
        left: &[Panel],
        right: &[Panel],
        hidden: &[Panel],
        scroll: f32,
    ) -> Self {
        let x = body.x + SETTINGS_PAD;
        let w = (body.w - SETTINGS_PAD * 2.0).max(0.0);
        let mut layout = PanelsLayout {
            body_h: body.h,
            ..PanelsLayout::default()
        };
        let mut y = body.y - scroll;

        for (column, panels, title) in [
            (Some(Column::Left), left, t!("settings-panels-left")),
            (Some(Column::Right), right, t!("settings-panels-right")),
            (None, hidden, t!("settings-panels-hidden")),
        ] {
            if panels.is_empty() {
                continue;
            }
            if !layout.headings.is_empty() {
                y += SETTINGS_GROUP_GAP;
            }
            let heading = Rect::new(x + SETTINGS_INSET, y, w, SETTINGS_TITLE);
            layout.headings.push((heading, title));
            y += SETTINGS_TITLE;

            let top = y;
            for (index, &panel) in panels.iter().enumerate() {
                if index > 0 {
                    let inner = (w - SETTINGS_INSET * 2.0).max(0.0);
                    layout
                        .separators
                        .push(Rect::new(x + SETTINGS_INSET, y, inner, 1.0));
                }
                let row = Rect::new(x, y, w, SETTINGS_ROW);
                let handle = Rect::new(
                    row.x + SETTINGS_INSET,
                    row.y + (row.h - 20.0) / 2.0,
                    20.0,
                    20.0,
                );
                // 按钮从右往左排：隐藏、移到另一栏、下移、上移。
                let button = |i: usize| {
                    let right = row.right() - SETTINGS_INSET - (i as f32) * (BTN + BTN_GAP);
                    Rect::new(right - BTN, row.y + (row.h - BTN) / 2.0, BTN, BTN)
                };
                let entry = PanelRow {
                    panel,
                    column,
                    row,
                    handle,
                    up: button(0),
                    down: button(1),
                    side: button(2),
                    hide: button(3),
                };
                match column {
                    Some(Column::Left) => layout.left.push(entry),
                    Some(Column::Right) => layout.right.push(entry),
                    None => layout.hidden.push(entry),
                }
                y += SETTINGS_ROW;
            }
            layout.groups.push(Rect::new(x, top, w, y - top));
        }

        // 右栏的宽度：最后一组下面一行。
        if !layout.groups.is_empty() {
            y += SETTINGS_GROUP_GAP;
        }
        let width = Rect::new(x, y, w, SETTINGS_ROW);
        let width_up = Rect::new(
            width.right() - SETTINGS_INSET - BTN,
            width.y + (width.h - BTN) / 2.0,
            BTN,
            BTN,
        );
        let width_down = Rect::new(width_up.x - BTN_GAP - BTN, width_up.y, BTN, BTN);
        layout.width = width;
        layout.width_down = width_down;
        layout.width_up = width_up;
        y += SETTINGS_ROW;

        // 多工作区模式：宽度行下面一行，右端是开关。
        y += SETTINGS_GROUP_GAP;
        let multi = Rect::new(x, y, w, SETTINGS_ROW);
        let toggle = Rect::new(
            multi.right() - SETTINGS_INSET - SETTINGS_SWITCH_W,
            multi.y + (multi.h - SETTINGS_SWITCH_H) / 2.0,
            SETTINGS_SWITCH_W,
            SETTINGS_SWITCH_H,
        );
        layout.multi_workspace = multi;
        layout.multi_toggle = toggle;
        y += SETTINGS_ROW;

        layout.content_h = y + scroll - body.y + SETTINGS_BOTTOM;
        layout
    }

    /// 返回列表的最大滚动量（像素）。
    pub fn max_scroll(&self) -> f32 {
        (self.content_h - self.body_h).max(0.0)
    }
}
