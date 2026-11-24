//! 左右两栏的绘制：项目名一行、各个面板的标题行和正文。
//!
//! 左栏和右栏结构一样：栏里从上到下排着若干面板，每个面板有自己的标题行（左边是图标和
//! 名字，右端是这个面板的工具）和正文。中间一栏固定是编辑区和终端。

use super::*;
use winc_config::Panel;

use crate::layout::Column;

/// 绘制左右两栏。
///
/// 左栏先画（它在左边），右栏后画。
pub(in crate::view) fn draw_columns(app: &mut App, frame: &mut Frame<'_>) {
    draw_column(app, frame, Column::Left);
    draw_column(app, frame, Column::Right);
}

/// 绘制一栏里的各个面板。
fn draw_column(app: &mut App, frame: &mut Frame<'_>, column: Column) {
    let (slots, count) = match column {
        Column::Left => (app.layout.left_panels, app.layout.left_count),
        Column::Right => (app.layout.right_panels, app.layout.right_count),
    };
    if column == Column::Left {
        // 工作区标签条只在左栏：它连着项目菜单（切换目录、打开目录）。
        //
        // 左栏没有面板时也要画：标签条是切换工作区的地方，不能跟着面板一起没了。
        sidebar::draw_workspaces(app, frame);
        if count == 0 {
            let area = Rect::new(
                app.layout.sidebar.x,
                app.layout.sidebar_header.bottom(),
                app.layout.sidebar.w,
                (app.layout.sidebar.bottom() - app.layout.sidebar_header.bottom()).max(0.0),
            );
            draw_empty_state(
                app,
                frame,
                (area, area),
                t!("workspace-sidebar-no-panels"),
                None,
            );
        }
    }
    if count == 0 {
        return;
    }
    // 有输入框的面板返回光标的位置，供输入法候选框定位。
    let mut caret = None;
    for slot in &slots[..count] {
        match slot.panel {
            Some(Panel::Files) => sidebar::draw_files_panel(app, frame, *slot),
            Some(Panel::Commit) => caret = git::commit::draw_commit_panel(app, frame),
            // 全局搜索改用浮在编辑区上的弹窗，不在两栏里绘制。
            Some(Panel::Search) => {}
            None => {}
        }
    }
    if let Some(caret) = caret {
        app.ime.cursor = Some(caret);
    }
}

/// 绘制窗口右上角的右栏开关按钮：收起或展开右栏（Git 等面板）。
///
/// 右栏里没有面板时不绘制。按钮画在卡片圆角之上：它压在右栏卡片（或中间一栏）的右上角。
pub(in crate::view) fn draw_right_button(app: &App, frame: &mut Frame<'_>) {
    let button = app.layout.right_button;
    if button.w <= 0.0 {
        return;
    }
    let p = app.palette;
    let hovered = app.pointer.hover == Hover::RightButton;
    let on = !app.panels.right_hidden;
    // 按钮在窗口右上角，不在中间那一栏的顶栏里：按窗口裁剪。
    draw_button(frame, p, button, app.layout.window, hovered, on);
    let color = if hovered { p.text } else { p.text_muted };
    icons::panel(frame, center(button), true, color, button);
}

/// 绘制面板标题行左边的图标和名字，返回名字右端的横坐标。
///
/// 标题行右端留给这个面板自己的工具；返回的横坐标是工具不该越过的地方。
pub(in crate::view) fn draw_panel_title(
    app: &App,
    frame: &mut Frame<'_>,
    header: Rect,
    panel: Panel,
) -> f32 {
    let p = app.palette;
    let icon = (header.x + 10.0, center(header).1);
    let color = p.text_muted;
    match panel {
        Panel::Files => icons::files(frame, icon, 0.85, color, header),
        Panel::Search => icons::search(frame, icon, 0.85, color, header),
        Panel::Commit => icons::git(frame, icon, 0.85, color, header),
    }
    let x = icon.0 + icons::FILE_WIDTH + 6.0;
    ui_text(frame, app, x, header, panel_name(panel), color, header)
}

/// 面板标题行里显示的名字。
pub(in crate::view) fn panel_name(panel: Panel) -> &'static str {
    match panel {
        Panel::Files => t!("workspace-sidebar-files"),
        Panel::Search => t!("workspace-sidebar-search"),
        Panel::Commit => t!("workspace-sidebar-commit"),
    }
}
