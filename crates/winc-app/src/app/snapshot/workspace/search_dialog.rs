//! 全局搜索弹窗的测试：弹窗浮在编辑区上方、随编辑区收窄，条件的绘制与替换行的展开。

use winit::keyboard::{Key, NamedKey};

use super::*;
use crate::keymap::Action;

/// 全局搜索浮在编辑区上方：侧栏不动，卡片在编辑区里。
#[test]
fn the_global_search_opens_a_dialog_over_the_editor() {
    let dir = search_folder("global-dialog");
    let mut app = app_with(&dir, false);
    paint(&mut app);

    // ⌘⇧F 打开弹窗：侧栏仍停在文件树，焦点在查找输入框。
    app.run_action(Action::FindInFiles);
    assert!(app.search_open());
    assert_eq!(app.panels.sidebar.view, Panel::Files);
    assert_eq!(app.focus, Focus::GlobalQuery);

    // 卡片浮在编辑区上方，不超出编辑区。
    paint(&mut app);
    let card = app.layout.search_card;
    assert!(card.h > 0.0);
    assert!(card.y >= app.layout.editor.y);
    assert!(card.x >= app.layout.editor.x);
    assert!(card.right() <= app.layout.editor.right());

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 打开一处匹配时，匹配行落在卡片下方的可见区域里，不被卡片盖住。
#[test]
fn opening_a_result_scrolls_it_below_the_dialog() {
    let dir = search_folder("global-dialog-scroll");
    // 足够长的文件：匹配行在文档中间，编辑区能滚动。
    let mut text = String::new();
    for i in 0..100 {
        text.push_str(&format!("let value_{i} = {i};\n"));
    }
    text.push_str("let config = 1;\n");
    for i in 0..100 {
        text.push_str(&format!("let other_{i} = {i};\n"));
    }
    std::fs::write(dir.join("src/long.rs"), text).unwrap();
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    app.panels.global.query.insert("config");
    app.run_global_search();
    app.panels.global.engine.wait();
    finish_search(&mut app);

    // 打开长文件里的一处匹配：匹配行落在卡片下方的可见区域里。
    let index = app
        .panels
        .global
        .rows()
        .iter()
        .position(|r| {
            matches!(r, ResultRow::Match(f, _) if app.panels.global.engine.results[*f].relative.ends_with("long.rs"))
        })
        .unwrap();
    let (file, m) = match app.panels.global.rows()[index] {
        ResultRow::Match(f, m) => (f, m),
        _ => unreachable!(),
    };
    let line = app.panels.global.engine.results[file].matches[m].line;
    app.open_result(index, false);
    paint(&mut app);

    let doc = app.editor.active().unwrap();
    let row = doc.row_of(line) as f64;
    let y = app.layout.text.y + (row - doc.scroll_line) as f32 * app.fonts.line_height;
    assert!(
        y >= app.layout.search_card.bottom(),
        "匹配行 {y} 在卡片下方吗？卡片下沿 {}",
        app.layout.search_card.bottom()
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

/// `Esc` 关掉弹窗，⌘⇧F 再按一次也关掉。
#[test]
fn escape_closes_the_dialog_and_the_shortcut_toggles_it() {
    let dir = search_folder("global-dialog-keys");
    let mut app = app_with(&dir, false);
    paint(&mut app);

    app.run_action(Action::FindInFiles);
    assert!(app.search_open());
    app.run_action(Action::FindInFiles);
    assert!(!app.search_open());

    app.run_action(Action::FindInFiles);
    assert!(app.search_open());
    // 结果列表里的 `Esc`：关掉弹窗，焦点回到编辑区。
    app.results_key(&Key::Named(NamedKey::Escape));
    assert!(!app.search_open());
    assert_eq!(app.focus, Focus::Editor);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 编辑区很窄时卡片随它收窄，输入框与开关都留在卡片里。
#[test]
fn the_dialog_narrows_with_the_editor() {
    let dir = search_folder("global-dialog-narrow");
    let mut app = app_with(&dir, false);
    // 侧栏很宽：留给编辑区的位置不多了。
    app.panels.sidebar.width = 900.0;
    app.relayout();
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    paint(&mut app);

    let card = app.layout.search_card;
    assert!(card.w < 720.0, "卡片没收窄：{}", card.w);
    assert!(card.right() <= app.layout.editor.right());
    assert!(card.bottom() <= app.layout.editor.bottom());

    // 查找行里的开关都在卡片内。
    let parts = app.panel_parts();
    for rect in [
        parts.case_toggle,
        parts.word_toggle,
        parts.regex_toggle,
        parts.details,
    ] {
        assert!(rect.right() <= card.right(), "开关画到卡片外了");
        assert!(rect.x >= card.x);
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 重新打开弹窗时，卡片高度按当前内容算，不沿用上一次的高度。
#[test]
fn reopening_the_dialog_uses_the_current_content_height() {
    let dir = search_folder("global-dialog-height");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    app.panels.global.query.set_text("config");
    app.run_global_search();
    finish_search(&mut app);
    let tall = app.layout.search_card.h;
    assert!(tall > 200.0, "结果多时卡片应当更高：{tall}");

    // 关掉弹窗：卡片高度动画不能留着，否则重新打开时按旧高度画。
    app.panels.global.card_motion = Some(PickerMotion {
        from: 300.0,
        to: 300.0,
        clock: crate::app::animation::Clock::frozen(
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(50),
        ),
    });
    app.close_global_search();
    paint(&mut app);
    assert!(
        app.panels.global.card_motion.is_none(),
        "关闭后还留着卡片高度动画"
    );
    app.run_action(Action::FindInFiles);
    app.panels.global.query.set_text("zzzz");
    app.run_global_search();
    finish_search(&mut app);
    paint(&mut app);
    let short = app.layout.search_card.h;
    assert!(
        short < tall,
        "卡片高度没有跟着内容变：{short}（上一次 {tall}）"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 没打开目录时按 ⌘⇧F：状态栏提示，不弹出空弹窗。
#[test]
fn the_dialog_needs_an_open_folder() {
    let mut app = app_with(&PathBuf::from("nonexistent"), false);
    paint(&mut app);
    app.status.message = None;

    app.run_action(Action::FindInFiles);
    assert!(!app.search_open());
    assert!(app.status.message.is_some(), "没有提示先打开目录");

    std::fs::remove_dir_all("nonexistent").ok();
}

/// 点卡片外的编辑区：弹窗关掉，这次点击仍然落在编辑区（光标挪到点中的那一行）。
#[test]
fn clicking_outside_the_dialog_reaches_the_editor() {
    let dir = search_folder("global-dialog-outside");
    let mut app = app_with(&dir.join("Cargo.toml"), false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    paint(&mut app);

    let text = app.layout.text;
    let at = (text.x + 40.0, text.y + 3.0 * app.fonts.line_height);
    app.on_mouse_move(at.0, at.1);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);

    assert!(!app.search_open());
    assert_eq!(app.focus, Focus::Editor);
    let doc = app.editor.active().unwrap();
    // 光标落在点中的那一行：这次点击没有被弹窗吞掉。
    assert_eq!(doc.text().pos_to_line_col(doc.selection.head).0, 2);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// ⌘⇧H 是「打开替换」：弹窗已经开着时展开替换行，不是把它关掉。
#[test]
fn the_replace_shortcut_expands_the_open_dialog() {
    let dir = search_folder("global-dialog-replace");
    let mut app = app_with(&dir.join("Cargo.toml"), false);
    paint(&mut app);

    app.run_action(Action::FindInFiles);
    assert!(app.search_open());
    app.panels.global.query.set_text("config");
    app.run_action(Action::ReplaceInFiles);
    assert!(app.search_open(), "⌘⇧H 把弹窗关掉了");
    assert!(app.panels.global.replace_open, "替换行没有展开");
    assert_eq!(app.focus, Focus::GlobalReplace);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 弹窗没开时用「下一处匹配」，焦点留在编辑区：结果列表不在屏幕上，焦点不能落进去。
#[test]
fn navigating_results_without_the_dialog_keeps_the_focus_in_the_editor() {
    let dir = search_folder("global-dialog-next");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    app.panels.global.query.set_text("config");
    app.run_global_search();
    finish_search(&mut app);
    app.close_global_search();
    paint(&mut app);

    app.run_action(Action::NextResult);
    assert_eq!(app.focus, Focus::Editor);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 编辑区很矮时条件区仍要完整地画在卡片里：放不下的只是结果列表。
#[test]
fn the_conditions_stay_inside_the_card_when_the_editor_is_short() {
    let dir = search_folder("global-dialog-short");
    let mut app = app_with(&dir, false);
    // 终端占掉一大截，编辑区变矮。
    app.terminal.height = 320.0;
    app.set_terminal_open(true);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    // 查找与替换都折成多行：条件区比留给卡片的高度还高。
    let long = "word ".repeat(300);
    app.panels.global.query.set_text(&long);
    app.panels.global.replace.set_text(&long);
    app.panels.global.replace_open = true;
    app.panels.global.details_open = true;
    app.run_global_search();
    finish_search(&mut app);
    paint(&mut app);

    let card = app.layout.search_card;
    let parts = app.panel_parts();
    assert!(parts.query.bottom() <= card.bottom(), "查找框画到卡片外了");
    assert!(
        parts.replace.bottom() <= card.bottom(),
        "替换框画到卡片外了"
    );
    assert!(
        parts.exclude.bottom() <= card.bottom(),
        "排除框画到卡片外了"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 弹窗里的输入框和结果列表都画在卡片内。
#[test]
fn the_dialog_draws_its_fields() {
    let dir = search_folder("global-dialog-draw");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    paint(&mut app);

    // 查找输入框有焦点时，输入法光标落在框里。
    assert!(app.ime.cursor.is_some());

    // 卡片里的几何与卡片一致：结果列表不超出卡片。
    let parts = app.panel_parts();
    let card = app.layout.search_card;
    assert!(parts.query.w > 0.0);
    assert!(parts.results.bottom() <= card.bottom() + 1.0);
    assert!(parts.results.right() <= card.right());

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 点卡片外关闭弹窗：弹窗浮在编辑区上方，点它挡住的编辑区就是关掉它。
#[test]
fn clicking_outside_the_dialog_closes_it() {
    let dir = search_folder("global-dialog-outside-2");
    let mut app = app_with(&dir.join("Cargo.toml"), false);
    paint(&mut app);
    app.run_action(Action::FindInFiles);
    paint(&mut app);
    assert!(app.search_open());

    // 点在卡片外的编辑区：弹窗关闭，焦点回到编辑区。
    let at = (
        app.layout.editor.x + 40.0,
        app.layout.editor.bottom() - 40.0,
    );
    app.on_mouse_move(at.0, at.1);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    assert!(!app.search_open());
    assert_eq!(app.focus, Focus::Editor);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn long_queries_and_replacements_grow_their_boxes() {
    let dir = search_folder("global-grow");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.open_global_search(false);
    app.panels.global.replace_open = true;
    settle_search(&mut app);
    let one = app.panel_parts();
    assert_eq!(one.query.h, crate::layout::PANEL_FIELD);

    // 长文字折成多行，框变高，下方的行跟着下移；开关留在第一行。
    let long = "word ".repeat(60);
    app.panels.global.query.insert(&long);
    app.panels.global.replace.insert(&long);
    settle_search(&mut app);
    let tall = app.panel_parts();
    assert!(tall.query.h > one.query.h);
    assert!(tall.replace.h > one.replace.h);
    assert_eq!(
        tall.replace.y,
        tall.query.bottom() + (one.replace.y - one.query.bottom())
    );
    assert_eq!(tall.case_toggle.y, one.case_toggle.y);
    assert!(tall.results.y > one.results.y);

    // 高度有上限，更长的文字在框内滚动。
    app.panels.global.query.insert(&long.repeat(20));
    settle_search(&mut app);
    let capped = app.panel_parts();
    assert_eq!(capped.query.h, crate::layout::PANEL_FIELD + 5.0 * 20.0);

    // 点第二行的文字：光标落在第二行，不是第一行。
    let parts = app.panel_parts();
    app.on_mouse_move(parts.query.x + 60.0, parts.query.y + 40.0);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    assert_eq!(app.focus, Focus::GlobalQuery);
    paint(&mut app);
    assert!(app.panels.global.query.caret() > 0);

    // 清空后恢复一行。
    app.panels.global.query = TextField::default();
    app.panels.global.replace = TextField::default();
    settle_search(&mut app);
    assert_eq!(app.panel_parts().query.h, one.query.h);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// 绘制到输入框高度的过渡结束。
fn settle_search(app: &mut App) {
    paint(app);
    app.panels.global.field_motion = None;
    paint(app);
}
