//! 设置中「面板」页的绘制：左右两栏里各有哪些面板、面板的顺序，以及右栏的宽度。
//!
//! 每组里逐行列出面板：行首是拖动把手（按住可以拖到别的位置或另一栏），中间是面板的
//! 名字，行尾是几个按钮（上移、下移、移到另一栏、隐藏）。最后一组是隐藏的面板，最下面
//! 一行是右栏的宽度。

use super::*;
use crate::app::settings::panels::{PanelRowButton, dragging_row};
use crate::layout::{Column, PanelRow, PanelsLayout};
use winc_config::Panel;

/// 绘制「面板」页。
pub(super) fn draw(app: &mut App, frame: &mut Frame<'_>, body: Rect) {
    let layout = app.panels_layout();
    let p = app.palette;

    for &(rect, title, _) in &layout.headings {
        ui_text_small(frame, app, rect.x, rect, title, p.text_dim, body);
    }
    for &group in &layout.groups {
        frame.rect_clipped(group, p.group_bg, RADIUS_BAR, body);
    }
    for &line in &layout.separators {
        frame.rect_clipped(line, p.separator, 0.0, body);
    }

    // 正在拖动的那一行跟着鼠标走，画在其他行之上。
    let drag = app.settings.panel_drag;
    for row in layout
        .left
        .iter()
        .chain(&layout.right)
        .chain(&layout.hidden)
    {
        if row.row.bottom() < body.y || row.row.y > body.bottom() {
            continue;
        }
        if drag.is_some_and(|d| d.panel == row.panel) {
            continue;
        }
        draw_row(app, frame, row, body);
    }
    if let Some(drag) = drag
        && let Some(row) = dragging_row(&layout, drag)
    {
        // 拖动中的行跟着鼠标走：它离开原来的位置，跟着指针。
        let moved = Rect::new(row.row.x, drag.at.1 - drag.grab, row.row.w, row.row.h);
        frame.with_opacity(0.75, |frame| {
            let moved = Rect::new(moved.x, moved.y, moved.w, moved.h);
            draw_row_at(app, frame, &row, moved, body);
        });
        // 落点：一条横线，标出松手后面板会插在哪儿。
        if let Some(line) = drop_line(app, &layout, body) {
            frame.rect_clipped(line, p.accent, 1.0, body);
        }
    }

    draw_width_row(app, frame, &layout, body);
    draw_multi_workspace_row(app, frame, &layout, body);
}

/// 落点的插入线：面板会插在哪儿，标在那一行之间。
///
/// 拖动时指针不在任何行、标题上时没有落点（松手就取消）。
fn drop_line(app: &App, layout: &PanelsLayout, body: Rect) -> Option<Rect> {
    let (column, index) = app.panel_drop_target(layout)?;
    let y = match column {
        Some(column) => {
            let rows = match column {
                Column::Left => &layout.left,
                Column::Right => &layout.right,
            };
            match rows.get(index) {
                Some(row) => Some(row.row.y),
                // 放在这一栏的末尾：最后一行下面。
                None => rows.last().map(|row| row.row.bottom()),
            }
        }
        // 藏起来：标在隐藏组的第一行上面。
        None => layout.hidden.first().map(|row| row.row.y),
    }?;
    let x = body.x + SETTINGS_PAD;
    Some(Rect::new(
        x,
        y - 1.0,
        (body.w - 2.0 * SETTINGS_PAD).max(0.0),
        2.0,
    ))
}

/// 绘制一行：一个面板的名字、拖动把手和行尾的按钮。
fn draw_row(app: &App, frame: &mut Frame<'_>, row: &PanelRow, clip: Rect) {
    draw_row_at(app, frame, row, row.row, clip);
}

/// 绘制一行，行放在 `at` 处：拖动中的行跟着鼠标走时用。
fn draw_row_at(app: &App, frame: &mut Frame<'_>, row: &PanelRow, at: Rect, clip: Rect) {
    let p = app.palette;
    // 把手跟着行走：拖动中的行整体移动。
    let handle = Rect::new(
        at.x + SETTINGS_INSET,
        at.y + (at.h - 20.0) / 2.0,
        20.0,
        20.0,
    );
    let hovered = app.settings.hover == Some(SettingsPart::PanelHandle(row.panel));

    // 拖动把手：三道横线。
    let color = if hovered { p.text } else { p.text_dim };
    for i in 0..3 {
        let y = handle.y + 5.0 + i as f32 * 5.0;
        frame.rect_clipped(
            Rect::new(handle.x + 2.0, y, handle.w - 4.0, 1.5),
            color,
            0.75,
            clip,
        );
    }

    // 名字：面板的图标加名称。
    let icon = (
        at.x + SETTINGS_INSET + 28.0,
        icon_center_y(app, at, app.fonts.ui),
    );
    match row.panel {
        Panel::Files => icons::files(frame, icon, 0.85, p.text_muted, clip),
        Panel::Search => icons::search(frame, icon, 0.85, p.text_muted, clip),
        Panel::Commit => icons::git(frame, icon, 0.85, p.text_muted, clip),
    }
    let name = crate::view::workspace::columns::panel_name(row.panel);
    ui_text(frame, app, icon.0 + 16.0, at, name, p.text, clip);

    // 行尾的按钮。
    let shift = |rect: Rect| -> Rect {
        if rect.w <= 0.0 {
            return rect;
        }
        Rect::new(
            rect.x + (at.x - row.row.x),
            rect.y + (at.y - row.row.y),
            rect.w,
            rect.h,
        )
    };
    let mut button = |rect: Rect, action: PanelRowButton, icon: RowIcon| {
        // 没有这个按钮（如栏里只有一个面板时的上移、下移）：不画。
        if rect.w <= 0.0 {
            return;
        }
        let rect = shift(rect);
        let hovered = app.settings.hover == Some(SettingsPart::PanelButton(action));
        if hovered {
            frame.rect_clipped(rect, p.hover, RADIUS_ROW, clip);
        }
        let color = if hovered { p.text } else { p.text_muted };
        let (at, clip) = (center(rect), rect.intersect(clip));
        match icon {
            RowIcon::Chevron(direction) => icons::chevron(frame, at, 6.0, direction, color, clip),
            RowIcon::Close => icons::close(frame, at, 6.5, color, clip),
        }
    };
    button(
        shift(row.up),
        PanelRowButton::Up(row.panel),
        RowIcon::Chevron(icons::Direction::Up),
    );
    button(
        shift(row.down),
        PanelRowButton::Down(row.panel),
        RowIcon::Chevron(icons::Direction::Down),
    );
    // 藏起来的面板：两个按钮把它放回左栏、右栏。
    let hidden = row.column.is_none();
    if hidden {
        button(
            shift(row.side),
            PanelRowButton::Show(row.panel, Column::Left),
            RowIcon::Chevron(icons::Direction::Left),
        );
        button(
            shift(row.hide),
            PanelRowButton::Show(row.panel, Column::Right),
            RowIcon::Chevron(icons::Direction::Right),
        );
    } else {
        // 移到另一栏：箭头指向另一边。
        let to_right = row.column != Some(Column::Right);
        button(
            shift(row.side),
            PanelRowButton::Side(row.panel),
            RowIcon::Chevron(if to_right {
                icons::Direction::Right
            } else {
                icons::Direction::Left
            }),
        );
        button(
            shift(row.hide),
            PanelRowButton::Hide(row.panel),
            RowIcon::Close,
        );
    }
}

/// 面板设置页里行尾按钮的图标。
enum RowIcon {
    /// 箭头。
    Chevron(icons::Direction),
    /// 叉：藏起来。
    Close,
}

/// 绘制「右栏宽度」一行：左边是名称，右端是减号和加号。
/// 绘制「多工作区模式」一行：左边是名字，右端是开关。
fn draw_multi_workspace_row(app: &App, frame: &mut Frame<'_>, layout: &PanelsLayout, clip: Rect) {
    let row = layout.multi_workspace;
    if row.bottom() < clip.y || row.y > clip.bottom() {
        return;
    }
    let p = app.palette;
    ui_text(
        frame,
        app,
        row.x + SETTINGS_INSET,
        row,
        t!("settings-multi-workspace"),
        p.text,
        clip,
    );
    let on = if app.switch_on(Setting::MultiWorkspace) {
        1.0
    } else {
        0.0
    };
    // 切换途中的开关从原状态过渡到新状态。
    let t = match app.settings.switch_slide {
        Some((Setting::MultiWorkspace, clock)) => clock.between(1.0 - on, on),
        _ => on,
    };
    super::controls::switch(frame, p, layout.multi_toggle, t, clip);
}

fn draw_width_row(app: &App, frame: &mut Frame<'_>, layout: &PanelsLayout, clip: Rect) {
    let p = app.palette;
    let row = layout.width;
    if row.bottom() < clip.y || row.y > clip.bottom() {
        return;
    }
    ui_text(
        frame,
        app,
        row.x + SETTINGS_INSET,
        row,
        t!("settings-panels-width"),
        p.text,
        clip,
    );
    let width = app.panels.layout.right_width.round() as i32;
    let text = format!("{width}");
    let text_x = row.right()
        - SETTINGS_INSET
        - 2.0 * (26.0 + 4.0)
        - 8.0
        - frame.measure(FontKind::Ui, app.fonts.ui, &text);
    ui_text(frame, app, text_x, row, &text, p.text_muted, clip);

    for (rect, action) in [
        (layout.width_down, PanelRowButton::WidthDown),
        (layout.width_up, PanelRowButton::WidthUp),
    ] {
        let hovered = app.settings.hover == Some(SettingsPart::PanelButton(action));
        if hovered {
            frame.rect_clipped(rect, p.hover, RADIUS_ROW, clip);
        }
        let color = if hovered { p.text } else { p.text_muted };
        let (x, y) = center(rect);
        if action == PanelRowButton::WidthDown {
            icons::minus(frame, (x, y), 6.0, color, rect.intersect(clip));
        } else {
            icons::plus(frame, (x, y), 8.0, color, rect.intersect(clip));
        }
    }
}
