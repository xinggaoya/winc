//! 全局搜索弹窗与「在文件中查找的对话框」的测试。
//!
//! 全局搜索弹窗浮在编辑区上方、随编辑区收窄，条件的绘制与替换行的展开；在文件中查找的
//! 对话框搜索、选择、打开、移动和改变大小、按设置打开、放进底部面板、文件掩码和范围、
//! 预览的缓存。

use winit::keyboard::{Key, NamedKey};

use super::*;
use crate::keymap::{Action, Context};

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

/// 在文件中查找的对话框：搜索、方向键选择、回车打开并选中匹配；拖动空白处移动、拖动边缘改变
/// 大小；点编辑区不关闭，Esc 关闭。
#[test]
fn the_search_dialog_finds_moves_and_stays_open() {
    let dir = search_folder("search-dialog");
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    let search = |app: &mut App, text: &str| {
        app.search_dialog.query.set_text(text);
        app.search_dialog_changed(true);
        app.search_dialog.engine.wait();
        app.poll_search_dialog(Instant::now());
        paint(app);
        app.search_dialog.hits.len()
    };

    app.open_search_dialog();
    assert!(app.search_dialog.open);
    assert_eq!(app.focus, Focus::SearchDialog);
    assert_eq!(app.key_context(), Context::Panel);
    assert!(search(&mut app, "load_config") >= 2);
    app.toggle_search_dialog_option(crate::layout::PanelButton::Regex);
    assert!(search(&mut app, "load_conf.g") >= 2);
    app.toggle_search_dialog_option(crate::layout::PanelButton::Regex);
    assert_eq!(search(&mut app, "load_conf.g"), 0);
    assert!(search(&mut app, "load_config") >= 2);

    // 拖动标题行的空白处移动；拖动右下角改变大小，不小于最小大小。
    let parts = app.search_dialog_parts();
    let (x, y) = (parts.header.x + 300.0, parts.header.y + 14.0);
    app.on_mouse_move(x, y);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_move(x - 40.0, y + 30.0);
    app.on_mouse_up(MouseButton::Left);
    let moved = app.search_dialog_rect();
    assert_eq!(
        (moved.x, moved.y),
        (parts.card.x - 40.0, parts.card.y + 30.0)
    );
    let corner = (moved.right() - 2.0, moved.bottom() - 2.0);
    app.on_mouse_move(corner.0, corner.1);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_move(corner.0 - 2000.0, corner.1 - 2000.0);
    app.on_mouse_up(MouseButton::Left);
    let small = app.search_dialog_rect();
    assert_eq!((small.x, small.y), (moved.x, moved.y));
    assert!(small.w >= 480.0 && small.h >= 360.0 && small.w < moved.w);

    // 点对话框以外的编辑区：焦点离开，对话框不关闭。
    let editor = app.layout.editor;
    super::click(&mut app, (editor.right() - 20.0, editor.bottom() - 20.0));
    assert!(app.search_dialog.open);
    assert_ne!(app.focus, Focus::SearchDialog);

    // 再打开：沿用位置；回车打开选中的一处并选中匹配，对话框关闭。
    app.open_search_dialog();
    assert_eq!(app.search_dialog_rect(), small);
    app.search_dialog_select(1);
    app.search_dialog_accept();
    paint(&mut app);
    paint(&mut app);
    assert!(!app.search_dialog.open);
    let doc = app.editor.active().unwrap();
    assert_eq!(
        doc.selection.head.abs_diff(doc.selection.anchor),
        "load_config".len()
    );

    // Esc 关闭。
    app.open_search_dialog();
    app.close_search_dialog();
    assert!(!app.search_dialog.open);
    assert_eq!(app.focus, Focus::Editor);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 「在文件中查找」按设置打开侧栏或对话框；没有设置时按键位方案。
#[test]
fn find_in_files_follows_the_search_style() {
    use winc_config::SearchStyle;
    let dir = search_folder("find-in-files-style");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    // 本仓库默认用全局搜索弹窗：设置里没写时就是它。
    assert_eq!(app.search_style(), SearchStyle::Sidebar);

    app.config.find_in_files = Some(SearchStyle::Sidebar);
    app.run_action(Action::FindInFiles);
    assert!(!app.search_dialog.open);
    assert_eq!(app.focus, Focus::GlobalQuery);
    app.config.find_in_files = Some(SearchStyle::Dialog);
    app.run_action(Action::FindInFiles);
    assert!(app.search_dialog.open);
    assert_eq!(app.focus, Focus::SearchDialog);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 「在底部显示」把结果放进底部面板的「搜索」标签：对话框关闭，结果和选中项留着；方向键选择、
/// 回车打开，结果仍在；关闭标签后面板里没有终端时一起收起。
#[test]
fn search_results_move_to_the_bottom_panel() {
    let dir = search_folder("search-dock");
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("load_config");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    paint(&mut app);
    let count = app.search_dialog.hits.len();
    assert!(count >= 2);

    app.dock_search_results();
    paint(&mut app);
    assert!(!app.search_dialog.open);
    assert!(app.docked_search_shown() && app.terminal.open);
    assert_eq!(app.focus, Focus::SearchDock);
    assert_eq!(app.search_dialog.hits.len(), count);
    let dock = app.docked_parts().unwrap();
    let row = dock.list.row(1, app.search_dialog.scroll);
    assert_eq!(
        app.docked_part_at(row.x + 40.0, row.y + 4.0),
        Some(crate::app::workspace::search_dialog::DialogPart::Row(1))
    );

    // 选中第二条并打开：文件打开、匹配选中，结果仍在面板里。
    app.search_dialog_select(1);
    app.search_dialog_accept();
    paint(&mut app);
    paint(&mut app);
    let doc = app.editor.active().unwrap();
    assert_eq!(
        doc.selection.head.abs_diff(doc.selection.anchor),
        "load_config".len()
    );
    assert!(app.docked_search_shown());

    // 「在对话框中打开」：查询不变，选中的一行在对话框的列表中可见。
    use crate::app::workspace::search_dialog::DialogPart;
    app.search_dialog.scroll = 500.0;
    app.open_search_dialog_with(None);
    assert!(app.search_dialog.open);
    assert_eq!(app.search_dialog.query.text(), "load_config");
    let list = app.search_dialog_parts().list;
    let row = list.row(app.search_dialog.selected, app.search_dialog.scroll);
    assert!(row.y >= list.list.y && row.bottom() <= list.list.bottom());
    app.close_search_dialog();

    // 面板标题栏上有「在对话框中打开」；标签里的查询框可以直接改查询、重新搜索。
    paint(&mut app);
    let reopen = crate::app::terminal::HeaderHit::SearchDialog;
    assert!(app.hits.terminal.iter().any(|(_, h)| *h == reopen));
    let field = app.docked_parts().unwrap().query;
    app.press_docked_search(DialogPart::Query, field.x + 4.0, field.y + 4.0);
    assert!(app.docked_query_focused());
    app.edit_field(|f| {
        f.select_all();
        f.insert("struct Config")
    });
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    assert_eq!(app.search_dialog.hits.len(), 1);
    assert!(app.docked_search_shown() && !app.search_dialog.open);

    app.close_docked_search();
    assert!(!app.search_dialog.docked);
    assert!(!app.terminal.open);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 侧栏搜索面板顶部的「在对话框中打开」：带上面板的查询和搜索选项打开对话框。
#[test]
fn the_sidebar_search_opens_in_a_dialog() {
    use crate::layout::{PanelButton, PanelTool};
    let dir = search_folder("search-pop-out");
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    app.open_global_search(false);
    app.edit_field(|f| f.insert("load_config"));
    app.panels.global.whole_word = true;
    app.run_global_search();
    finish_search(&mut app);

    let parts = app.panel_parts();
    assert!(parts.tools.iter().any(|(_, t)| *t == PanelTool::OpenDialog));
    app.press_panel_button(PanelButton::Tool(PanelTool::OpenDialog));
    assert!(app.search_dialog.open);
    assert_eq!(app.focus, Focus::SearchDialog);
    assert_eq!(app.search_dialog.query.text(), "load_config");
    assert!(app.search_dialog.whole_word);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 对话框的文件掩码和范围：显示文件掩码时只搜索匹配的文件；「目录」只搜索当前文件所在的目录。
#[test]
fn the_search_dialog_filters_by_mask_and_directory() {
    use crate::app::workspace::search_dialog::DialogPart;
    let dir = search_folder("search-dialog-scope");
    let mut app = app_with(&dir, false);
    app.open_path(&dir.join("src/main.rs"));
    paint(&mut app);
    let files = |app: &mut App| {
        app.search_dialog.engine.wait();
        app.poll_search_dialog(Instant::now());
        let mut names: Vec<String> = (app.search_dialog.engine.results.iter())
            .map(|f| f.relative.clone())
            .collect();
        names.sort();
        names
    };
    app.open_search_dialog();
    app.search_dialog.query.set_text("config");
    app.search_dialog_changed(true);
    assert!(files(&mut app).iter().any(|n| n.ends_with("Cargo.toml")));

    // 文件掩码：只搜索 `*.toml`；隐藏后恢复。
    app.search_dialog.mask.set_text("*.toml");
    app.press_search_dialog(DialogPart::MaskToggle, 0.0, 0.0);
    assert_eq!(files(&mut app), ["Cargo.toml"]);
    app.press_search_dialog(DialogPart::MaskToggle, 0.0, 0.0);
    assert!(files(&mut app).len() > 1);

    // 目录：只搜索 src，不再有 Cargo.toml。
    app.press_search_dialog(DialogPart::Scope(1), 0.0, 0.0);
    assert!(app.search_dialog.in_dir);
    let names = files(&mut app);
    assert!(
        !names.is_empty() && names.iter().all(|n| !n.ends_with("Cargo.toml")),
        "{names:?}"
    );
    app.press_search_dialog(DialogPart::Scope(0), 0.0, 0.0);
    assert!(files(&mut app).iter().any(|n| n.ends_with("Cargo.toml")));

    // 在文件掩码中输入时点范围，焦点留在文件掩码中。
    app.press_search_dialog(DialogPart::MaskToggle, 0.0, 0.0);
    assert_eq!(app.focus, Focus::SearchMask);
    app.press_search_dialog(DialogPart::Scope(1), 0.0, 0.0);
    assert_eq!(app.focus, Focus::SearchMask);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 比预览区短的文件读一次后留在缓存里，重绘时不再读盘。
#[test]
fn a_short_file_preview_is_read_once() {
    let dir = search_folder("search-dialog-cache");
    std::fs::write(dir.join("src/tiny.rs"), "fn tiny_marker() {}\n").unwrap();
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("tiny_marker");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    let first = app.search_dialog_context().unwrap();
    assert_eq!(first.2, ["fn tiny_marker() {}"]);
    // 文件删掉后仍从缓存取到同样的几行。
    std::fs::remove_file(dir.join("src/tiny.rs")).unwrap();
    assert_eq!(app.search_dialog_context(), Some(first));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 换了查询而新的结果还没到时，列表留着上一次的结果（淡色），新的结果到了再换上并回到第一条。
#[test]
fn old_results_stay_until_new_ones_arrive() {
    let dir = search_folder("search-dialog-stale");
    let mut app = app_with(&dir, false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("config");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    let before = app.search_dialog.hits.len();
    assert!(before > 1);
    app.search_dialog_select(1);

    app.search_dialog.query.set_text("load_config");
    app.search_dialog_changed(true);
    assert!(app.search_dialog.stale);
    assert_eq!(app.search_dialog.hits.len(), before);
    // 这时回车不打开上一次的结果。
    let opened = app.editor.documents.len();
    app.search_dialog_accept();
    assert!(app.search_dialog.open);
    assert_eq!(app.editor.documents.len(), opened);

    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    assert!(!app.search_dialog.stale);
    assert!(app.search_dialog.hits.len() < before);
    assert_eq!(app.search_dialog.selected, 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 收起底部面板或关闭「搜索」标签时，焦点从标签里的查询或结果列表回到编辑区。
#[test]
fn hiding_the_search_tab_returns_focus_to_the_editor() {
    let dir = search_folder("search-dock-focus");
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("config");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    app.dock_search_results();
    paint(&mut app);
    let field = app.docked_parts().unwrap().query;
    let query = crate::app::workspace::search_dialog::DialogPart::Query;

    app.press_docked_search(query, field.x + 4.0, field.y + 4.0);
    assert!(app.docked_query_focused());
    app.set_terminal_open(false);
    assert_eq!(app.focus, Focus::Editor);

    app.set_terminal_open(true);
    paint(&mut app);
    app.press_docked_search(query, field.x + 4.0, field.y + 4.0);
    app.close_docked_search();
    assert_eq!(app.focus, Focus::Editor);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 对话框的样子（浅色和深色）：四周留同样的边距，标题、输入的文字和列表的图标在同一列，预览
/// 一直排到底边；侧栏搜索标题行上是「在对话框中打开」的按钮。
///
/// 运行：`cargo test -p winc-app shoot_search_dialog -- --ignored`。
#[test]
#[ignore = "截图，手动运行"]
fn shoot_search_dialog() {
    for dark in [false, true] {
        let dir = search_folder(if dark {
            "dialog-shot-dark"
        } else {
            "dialog-shot-light"
        });
        let mut app = app_with(&dir.join("src/main.rs"), dark);
        paint(&mut app);
        app.open_global_search(false);
        app.open_search_dialog();
        app.search_dialog.query.set_text("config");
        app.search_dialog_changed(true);
        app.search_dialog.engine.wait();
        app.poll_search_dialog(Instant::now());
        paint(&mut app);
        let theme = if dark { "dark" } else { "light" };
        shoot(&mut app, &format!("search-dialog-{theme}"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// 回归：开着在文件中查找的对话框时打开设置，鼠标在两者重叠处滚动：滚的是上面的设置，不是下面
/// 的对话框。
#[test]
fn the_wheel_scrolls_settings_over_the_search_dialog() {
    use winit::event::{MouseScrollDelta, TouchPhase};

    let dir = search_folder("dialog-under-settings");
    for i in 0..40 {
        std::fs::write(dir.join(format!("src/f{i}.rs")), "let config = 1;\n").unwrap();
    }
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("config");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    paint(&mut app);
    let list = app.search_dialog_parts().list.list;
    let at = (list.x + list.w / 2.0, list.y + list.h / 2.0);
    // 先确认这里滚得动对话框。
    app.on_mouse_move(at.0, at.1);
    app.on_wheel_event(MouseScrollDelta::LineDelta(0.0, -1.0), TouchPhase::Moved);
    assert!(app.search_dialog.scroll > 0.0);
    app.search_dialog.scroll = 0.0;

    app.open_settings();
    paint(&mut app);
    assert!(
        app.settings_layout().body.contains(at.0, at.1),
        "设置盖在对话框上"
    );
    app.on_mouse_move(at.0, at.1);
    app.on_wheel_event(MouseScrollDelta::LineDelta(0.0, -1.0), TouchPhase::Moved);
    assert_eq!(app.search_dialog.scroll, 0.0, "滚到了下面的对话框");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 在文件中查找的对话框里搜 `load_config`，等结果列出来。
fn search_dialog_with_results(name: &str) -> (App, std::path::PathBuf) {
    let dir = search_folder(name);
    let mut app = app_with(&dir.join("src/main.rs"), false);
    paint(&mut app);
    app.open_search_dialog();
    app.search_dialog.query.set_text("load_config");
    app.search_dialog_changed(true);
    app.search_dialog.engine.wait();
    app.poll_search_dialog(Instant::now());
    paint(&mut app);
    (app, dir)
}

/// 回归：在文件中查找的对话框盖在编辑区上，在它上面右键不弹出编辑区的菜单，光标不动。
#[test]
fn right_clicks_on_the_search_dialog_reach_no_menu() {
    let (mut app, dir) = search_dialog_with_results("search-dialog-menu");
    let list = app.search_dialog_parts().list.list;
    let at = (list.x + list.w / 2.0, list.y + list.h / 2.0);
    assert!(app.layout.editor.contains(at.0, at.1), "对话框盖在编辑区上");
    let caret = app.editor.active().unwrap().selection;
    right_click(&mut app, at);
    assert!(app.overlays.context_menu.is_none());
    assert_eq!(app.editor.active().unwrap().selection, caret);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 回归：底部面板的「搜索」标签盖在终端上，在它上面右键不弹出终端的菜单。
#[cfg(unix)]
#[test]
fn right_clicks_on_the_search_tab_reach_no_menu() {
    let (mut app, dir) = search_dialog_with_results("search-tab-menu");
    let _term = super::super::terminal::show_terminal(&mut app, "$ ");
    app.dock_search_results();
    paint(&mut app);
    assert!(app.docked_search_shown());
    let row = app
        .docked_parts()
        .unwrap()
        .list
        .row(0, app.search_dialog.scroll);
    let at = (row.x + 40.0, row.y + 4.0);
    assert!(
        app.layout.terminal.contains(at.0, at.1),
        "「搜索」标签盖在终端上"
    );
    right_click(&mut app, at);
    assert!(app.overlays.context_menu.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
