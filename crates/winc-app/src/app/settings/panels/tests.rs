//! 「面板」设置页的测试：拖动时的落点、行尾按钮。

use super::*;

use winc_config::Panel;

/// 页面正文的位置：够放下三组面板。
fn body() -> Rect {
    Rect::new(0.0, 0.0, 600.0, 400.0)
}

/// 拖到某一栏的标题上：落到那一栏的末尾，不是左栏。
#[test]
fn dropping_on_a_heading_lands_in_that_column() {
    let layout = PanelsLayout::compute(body(), &[Panel::Files], &[Panel::Commit], &[], 0.0);
    let (heading, _, column) = &layout.headings[1];
    assert_eq!(*column, Some(Column::Right), "右栏的标题");
    let target = drop_target(&layout, heading.x + 4.0, heading.y + 4.0);
    assert_eq!(
        target,
        Some((Some(Column::Right), usize::MAX)),
        "拖到右栏的标题上落到右栏末尾"
    );
}

/// 落在所有行与标题之外：取消拖动，不把面板藏起来。
#[test]
fn dropping_outside_every_row_cancels() {
    let layout = PanelsLayout::compute(body(), &[Panel::Files], &[Panel::Commit], &[], 0.0);
    assert_eq!(
        drop_target(&layout, 4.0, 398.0),
        None,
        "落在空白处不隐藏面板"
    );
}

/// 拖到「隐藏」那一组里：把它藏起来。
#[test]
fn dropping_on_the_hidden_group_hides_the_panel() {
    let layout = PanelsLayout::compute(body(), &[Panel::Files], &[], &[Panel::Commit], 0.0);
    let hidden = &layout.hidden[0];
    let target = drop_target(&layout, hidden.row.x + 4.0, hidden.row.y + 4.0);
    assert_eq!(target, Some((None, 0)), "拖到隐藏组里就是藏起来");
}

/// 一栏里只有一个面板：没有上移、下移可点。
#[test]
fn a_column_with_one_panel_has_no_move_buttons() {
    let layout = PanelsLayout::compute(body(), &[Panel::Files], &[Panel::Commit], &[], 0.0);
    let only = &layout.left[0];
    assert!(
        only.up.w <= 0.0 && only.down.w <= 0.0,
        "只有一个面板时不能上下移"
    );
    assert!(only.side.w > 0.0, "移到另一栏总是可以");
    assert!(only.hide.w > 0.0, "藏起来总是可以");
}
