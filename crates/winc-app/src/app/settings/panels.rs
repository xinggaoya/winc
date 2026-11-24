//! 「面板」设置页的操作：左右两栏里各有哪些面板、面板的顺序，以及拖动时的落点。
//!
//! 面板的排布记在 [`PanelLayout`] 里（见 [`crate::app::workspace::PanelLayout`]），改动后
//! 写回配置（`panels.left`、`panels.right` 和 `panels.rightWidth`）。

use super::*;
use crate::layout::{Column, PanelRow, PanelsLayout};

impl App {
    /// 配置里的面板排布变了：按新的配置摆好左右两栏，重新布局。
    pub(in crate::app) fn adopt_panels(&mut self) {
        let config = &self.config;
        let layout = &mut self.panels.layout;
        if let Some(left) = &config.panels_left {
            layout.left = crate::app::workspace::columns::without_search(left);
        }
        if let Some(right) = &config.panels_right {
            layout.right = crate::app::workspace::columns::without_search(right);
        }
        if let Some(width) = config.panel_right_width {
            layout.right_width = width;
        }
        self.relayout();
        self.request_redraw();
    }

    /// 左栏、右栏和隐藏的面板：按配置的顺序。
    pub(in crate::app) fn panel_groups(&self) -> (Vec<Panel>, Vec<Panel>, Vec<Panel>) {
        let layout = &self.panels.layout;
        let hidden = Panel::ALL
            .into_iter()
            .filter(|panel| !layout.left.contains(panel) && !layout.right.contains(panel))
            .collect();
        (layout.left.clone(), layout.right.clone(), hidden)
    }

    /// 「面板」页的几何。
    pub(crate) fn panels_layout(&self) -> PanelsLayout {
        let (left, right, hidden) = self.panel_groups();
        PanelsLayout::compute(
            self.settings_body(),
            &left,
            &right,
            &hidden,
            self.settings.scroll,
        )
    }

    /// 设置页正文的位置：标题下方可以滚动的区域。
    fn settings_body(&self) -> Rect {
        let layout = self.settings_layout();
        layout.body
    }

    /// 把面板 `panel` 移到 `column` 的第 `index` 处；`column` 为 `None` 时隐藏它。
    ///
    /// 已经在目标位置时不做任何事。
    pub(in crate::app) fn move_panel(
        &mut self,
        panel: Panel,
        column: Option<Column>,
        index: usize,
    ) {
        let layout = &mut self.panels.layout;
        layout.left.retain(|p| *p != panel);
        layout.right.retain(|p| *p != panel);
        if let Some(column) = column {
            let list = match column {
                Column::Left => &mut layout.left,
                Column::Right => &mut layout.right,
            };
            let index = index.min(list.len());
            list.insert(index, panel);
        }
        self.panels_changed();
    }

    /// 把面板 `panel` 在当前栏里上移或下移一位。
    pub(in crate::app) fn shift_panel(&mut self, panel: Panel, up: bool) {
        let layout = &mut self.panels.layout;
        let mut swapped = false;
        for list in [&mut layout.left, &mut layout.right] {
            let Some(index) = list.iter().position(|p| *p == panel) else {
                continue;
            };
            let target = if up {
                index.checked_sub(1)
            } else if index + 1 < list.len() {
                Some(index + 1)
            } else {
                None
            };
            if let Some(target) = target {
                list.swap(index, target);
                swapped = true;
            }
            break;
        }
        if swapped {
            self.panels_changed();
        }
    }

    /// 隐藏面板 `panel`，或把隐藏的面板放回右栏末尾。
    pub(in crate::app) fn toggle_panel_hidden(&mut self, panel: Panel) {
        let layout = &mut self.panels.layout;
        let hidden = !layout.left.contains(&panel) && !layout.right.contains(&panel);
        if hidden {
            layout.right.push(panel);
        } else {
            layout.left.retain(|p| *p != panel);
            layout.right.retain(|p| *p != panel);
        }
        self.panels_changed();
    }

    /// 右栏的宽度加或减一档。
    pub(in crate::app) fn step_right_width(&mut self, up: bool) {
        let step = 20.0;
        let range = winc_config::Config::PANEL_WIDTH_RANGE;
        let width = self.panels.layout.right_width;
        let width = if up { width + step } else { width - step };
        self.panels.layout.right_width = width.clamp(*range.start(), *range.end());
        self.panels_changed();
    }

    /// 面板的排布改了：写回配置，重新布局。
    pub(in crate::app) fn panels_changed(&mut self) {
        let layout = &self.panels.layout;
        self.config.panels_left = Some(layout.left.clone());
        self.config.panels_right = Some(layout.right.clone());
        self.config.panel_right_width = Some(layout.right_width);
        self.save_config();
        self.relayout();
        self.request_redraw();
    }
}

/// 拖动中的面板落在哪：栏和序号；落点不在任何地方时返回 `None`（取消拖动）。
///
/// 落在某一行上时是那一行的位置；落在组标题上是那一栏的末尾；落在「隐藏」组里是隐藏。
pub(crate) fn drop_target(
    layout: &PanelsLayout,
    x: f32,
    y: f32,
) -> Option<(Option<Column>, usize)> {
    for row in layout.left.iter().chain(&layout.right) {
        if row.row.contains(x, y) {
            let index = layout
                .left
                .iter()
                .position(|r| r.panel == row.panel)
                .or_else(|| layout.right.iter().position(|r| r.panel == row.panel))
                .unwrap_or(0);
            return Some((row.column, index));
        }
    }
    for row in &layout.hidden {
        if row.row.contains(x, y) {
            return Some((None, 0));
        }
    }
    // 落在组标题上：放到那一栏的末尾。标题记着自己对应哪一栏。
    for (heading, _, column) in &layout.headings {
        if heading.contains(x, y) {
            return Some((*column, usize::MAX));
        }
    }
    // 落在所有行与标题之外：没有落点，拖动取消。
    None
}

/// 「面板」页中正在拖动的面板。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PanelDrag {
    /// 正在拖动的面板。
    pub panel: Panel,
    /// 拖动开始时它在的栏。
    pub from: Option<Column>,
    /// 拖动开始时它在栏里的序号。
    pub index: usize,
    /// 按下时鼠标相对行上沿的距离。
    pub grab: f32,
    /// 鼠标当前的位置。
    pub at: (f32, f32),
}

impl App {
    /// 拖动中的面板落在哪：栏和序号。
    ///
    /// 落在某一行上时是那一行的位置；落在组标题上是那一栏的末尾；落在「隐藏」组里是隐藏。
    pub(crate) fn panel_drop_target(
        &self,
        layout: &PanelsLayout,
    ) -> Option<(Option<Column>, usize)> {
        drop_target(layout, self.pointer.mouse.0, self.pointer.mouse.1)
    }

    /// 把拖动中的面板放到落点。
    pub(in crate::app) fn drop_panel(&mut self, layout: &PanelsLayout) {
        let Some(drag) = self.settings.panel_drag else {
            return;
        };
        let Some((column, index)) = self.panel_drop_target(layout) else {
            // 落点不在任何地方：取消拖动，面板的位置不变。
            return;
        };
        // 落在自己身上时不动。
        if column == drag.from {
            return;
        }
        let index = if index == usize::MAX {
            match column {
                Some(Column::Left) => self.panels.layout.left.len(),
                Some(Column::Right) => self.panels.layout.right.len(),
                None => 0,
            }
        } else {
            index
        };
        self.move_panel(drag.panel, column, index);
    }
}

/// 「面板」页中一行是哪个按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelButton {
    /// 上移。
    Up(Panel),
    /// 下移。
    Down(Panel),
    /// 移到另一栏。
    Side(Panel),
    /// 隐藏或显示。
    Hide(Panel),
    /// 把藏着的面板放回某一栏。
    Show(Panel, Column),
    /// 右栏的宽度减一档。
    WidthDown,
    /// 右栏的宽度加一档。
    WidthUp,
}

impl App {
    /// 「面板」页中 `(x, y)` 处的按钮；不在任何按钮上时返回 `None`。
    pub(in crate::app) fn panel_button_at(
        &self,
        layout: &PanelsLayout,
        x: f32,
        y: f32,
    ) -> Option<PanelButton> {
        if layout.width_down.contains(x, y) {
            return Some(PanelButton::WidthDown);
        }
        if layout.width_up.contains(x, y) {
            return Some(PanelButton::WidthUp);
        }
        for row in layout
            .left
            .iter()
            .chain(&layout.right)
            .chain(&layout.hidden)
        {
            if row.up.contains(x, y) {
                return Some(PanelButton::Up(row.panel));
            }
            if row.down.contains(x, y) {
                return Some(PanelButton::Down(row.panel));
            }
            if row.side.contains(x, y) {
                return Some(PanelButton::Side(row.panel));
            }
            if row.hide.contains(x, y) {
                return Some(PanelButton::Hide(row.panel));
            }
        }
        None
    }
}

/// 「面板」页中正在拖动的那一行（用于绘制时让它跟着鼠标走）。
pub(crate) fn dragging_row(layout: &PanelsLayout, drag: PanelDrag) -> Option<PanelRow> {
    layout
        .left
        .iter()
        .chain(&layout.right)
        .chain(&layout.hidden)
        .find(|row| row.panel == drag.panel)
        .copied()
}

impl App {
    /// 「面板」页中 `(x, y)` 处的部件：拖动把手、行尾的按钮或多工作区模式的开关。
    pub(crate) fn panels_part_at(&self, x: f32, y: f32) -> Option<SettingsPart> {
        let layout = self.panels_layout();
        if layout.multi_toggle.contains(x, y) {
            return Some(SettingsPart::Switch(Setting::MultiWorkspace));
        }
        if let Some(button) = self.panel_button_at(&layout, x, y) {
            return Some(SettingsPart::PanelButton(button));
        }
        for row in layout
            .left
            .iter()
            .chain(&layout.right)
            .chain(&layout.hidden)
        {
            if row.handle.contains(x, y) {
                return Some(SettingsPart::PanelHandle(row.panel));
            }
        }
        None
    }

    /// 按下「面板」页中某一行行首的把手：开始拖动这一行。
    pub(crate) fn start_panel_drag(&mut self, panel: Panel, y: f32) {
        let layout = self.panels_layout();
        let row = layout
            .left
            .iter()
            .chain(&layout.right)
            .chain(&layout.hidden)
            .find(|row| row.panel == panel);
        let Some(row) = row else { return };
        let index = layout
            .left
            .iter()
            .position(|r| r.panel == panel)
            .or_else(|| layout.right.iter().position(|r| r.panel == panel));
        self.settings.panel_drag = Some(PanelDrag {
            panel,
            from: row.column,
            index: index.unwrap_or(0),
            grab: y - row.row.y,
            at: self.pointer.mouse,
        });
        self.pointer.drag = crate::app::Drag::PanelRow {
            panel,
            grab: y - row.row.y,
        };
        self.request_redraw();
    }

    /// 点击「面板」页中行尾的按钮。
    pub(crate) fn press_panel_layout_button(&mut self, button: PanelButton) {
        match button {
            PanelButton::Up(panel) => self.shift_panel(panel, true),
            PanelButton::Down(panel) => self.shift_panel(panel, false),
            PanelButton::Side(panel) => {
                let layout = &self.panels.layout;
                let to = if layout.left.contains(&panel) {
                    Some(Column::Right)
                } else {
                    Some(Column::Left)
                };
                let index = layout
                    .left
                    .iter()
                    .position(|p| *p == panel)
                    .or_else(|| layout.right.iter().position(|p| *p == panel))
                    .unwrap_or(0);
                self.move_panel(panel, to, index);
            }
            PanelButton::Hide(panel) => self.toggle_panel_hidden(panel),
            PanelButton::Show(panel, column) => {
                // 放回某一栏的末尾。
                let index = match column {
                    Column::Left => self.panels.layout.left.len(),
                    Column::Right => self.panels.layout.right.len(),
                };
                self.move_panel(panel, Some(column), index);
            }
            PanelButton::WidthDown => self.step_right_width(false),
            PanelButton::WidthUp => self.step_right_width(true),
        }
        self.request_redraw();
    }
}

#[cfg(test)]
mod tests;
