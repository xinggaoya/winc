//! 多工作区模式的测试：同一个窗口里同时打开多个项目，顶栏里切换，各自的标签、文件树和
//! 终端都留着。

use super::*;

use crate::app::NoticeAction;
use crate::keymap::Action;

/// 打开目录 `path`：多工作区模式下它成了窗口里的另一个工作区。
fn open_workspace(app: &mut App, path: &Path) {
    app.open_path(path);
}

/// 当前工作区打开的目录。
fn root(app: &App) -> PathBuf {
    app.editor.workspace.as_ref().unwrap().root.clone()
}

/// 把鼠标在工作区标签上停留的时间记为已满 `CLOSE_HOVER_DELAY`：关闭按钮出现，可以点击。
fn rest_on_chip(app: &mut App) {
    app.workspace_bar.rest = Some(std::time::Instant::now() - crate::theme::CLOSE_HOVER_DELAY);
    app.tick();
}

/// 打开的标签的名字。
fn names(app: &App) -> Vec<String> {
    app.editor
        .documents
        .iter()
        .map(|d| d.name().into_owned())
        .collect()
}

#[test]
fn opening_a_second_folder_adds_a_workspace_and_keeps_both_sets_of_tabs() {
    let a = search_folder("workspace-a");
    let b = search_folder("workspace-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    app.open_path(&root_a.join("src/main.rs"));

    // 只有一个工作区时，左栏顶部只有它一个标签。
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(app.workspace_bar.chips.len(), 1);

    // 打开第二个目录：它成了另一个工作区，顶栏里出现两个按钮。
    open_workspace(&mut app, &b);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 2);
    assert_eq!(app.workspaces.active, 1);
    assert_eq!(app.workspace_bar.chips.len(), 2);
    assert_eq!(root(&app), winc_config::files::canonicalize(&b).unwrap());

    // 第二个工作区里没有标签。
    assert!(names(&app).is_empty());
    app.open_path(&b.join("src/main.rs"));
    assert_eq!(names(&app), ["main.rs"]);

    // 切回第一个工作区：它的标签还在。
    app.switch_workspace(0);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 0);
    assert_eq!(root(&app), root_a);
    assert_eq!(names(&app), ["main.rs"]);

    // 再切回来：第二个工作区的标签也在。
    app.switch_workspace(1);
    paint(&mut app);
    assert_eq!(root(&app), winc_config::files::canonicalize(&b).unwrap());
    assert_eq!(names(&app), ["main.rs"]);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn the_workspace_bar_switches_between_the_workspaces() {
    let a = search_folder("workspace-bar-a");
    let b = search_folder("workspace-bar-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 左栏顶部每个工作区一个标签，末尾是「打开项目」与项目菜单两个按钮。
    let chips = app.workspace_bar.chips.clone();
    assert_eq!(chips.len(), 2);
    assert_eq!(chips[0].index, 0);
    assert_eq!(chips[1].index, 1);
    assert!(app.workspace_bar.add.w > 0.0);
    assert!(app.workspace_bar.more.w > 0.0);

    // 标签条放不下时滚到当前工作区：滚出去的那一个看不见也点不到，滚回来才能点。
    app.workspace_bar.scroll = 0.0;
    paint(&mut app);
    let chips = app.workspace_bar.chips.clone();

    // 点第一个工作区的按钮：切过去。
    let (x, y) = (
        chips[0].rect.x + 4.0,
        chips[0].rect.y + chips[0].rect.h / 2.0,
    );
    app.on_mouse_move(x, y);
    assert_eq!(app.pointer.hover, Hover::Workspace(0));
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 0);
    assert_eq!(root(&app), root_a);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn the_workspace_commands_switch_and_close_the_workspaces() {
    let a = search_folder("workspace-cmd-a");
    let b = search_folder("workspace-cmd-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    open_workspace(&mut app, &b);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 1);

    // 下一个工作区：回到第一个（只有两个，来回切）。
    app.run_action(Action::NextWorkspace);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 0);
    app.run_action(Action::PreviousWorkspace);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 1);

    // 关掉当前的工作区：切到相邻的一个。
    app.run_action(Action::CloseWorkspace);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(app.workspaces.active, 0);
    assert_eq!(root(&app), root_a);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn a_workspace_keeps_its_own_file_tree_and_git_state() {
    let a = search_folder("workspace-tree-a");
    let b = search_folder("workspace-tree-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    // 第一个工作区的文件树展开了一个目录。
    let row = row_of(&app, "src");
    app.editor.workspace.as_mut().unwrap().toggle(row);
    paint(&mut app);

    let expanded = app.editor.workspace.as_ref().unwrap().rows().len();
    open_workspace(&mut app, &b);
    paint(&mut app);
    // 第二个工作区的文件树是收着的：比第一个工作区的少两行。
    let other = app.editor.workspace.as_ref().unwrap().rows().len();
    assert!(other < expanded);

    app.switch_workspace(0);
    paint(&mut app);
    // 切回来时展开的目录还在。
    assert_eq!(
        app.editor.workspace.as_ref().unwrap().rows().len(),
        expanded
    );
    assert_eq!(root(&app), root_a);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn closing_the_last_workspace_closes_the_folder() {
    let a = search_folder("workspace-last");
    let mut app = app_with(&a, false);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);

    app.close_workspace(0);
    paint(&mut app);
    assert!(app.workspaces.slots.is_empty());
    assert!(app.editor.workspace.is_none());
    assert!(app.editor.documents.is_empty());

    let _ = std::fs::remove_dir_all(&a);
}

#[test]
fn the_multi_workspace_mode_can_be_turned_off() {
    let a = search_folder("workspace-off-a");
    let b = search_folder("workspace-off-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    app.config.multi_workspace = Some(false);
    paint(&mut app);

    // 单工作区模式：打开另一个目录就是换目录，原来那个不留在窗口里。
    app.open_path(&b);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(root(&app), winc_config::files::canonicalize(&b).unwrap());

    // 再打开第一个目录：还是只有一个工作区。
    app.open_path(&root_a);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(root(&app), root_a);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 工作区标签条住在左栏顶部一行：中间栏不因工作区让位。
#[test]
fn the_workspace_tabs_are_in_the_left_column() {
    let a = search_folder("workspace-tabs-left-a");
    let b = search_folder("workspace-tabs-left-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 标签条在左栏头部一行里。
    let chips = app.workspace_bar.chips.clone();
    assert_eq!(chips.len(), 2);
    let header = app.layout.sidebar_header;
    for chip in &chips {
        assert!(chip.rect.y >= header.y && chip.rect.bottom() <= header.bottom());
    }

    // 末尾两个按钮钉在左栏的右端，标签在它们左边滚动。
    assert!(app.workspace_bar.view.x >= header.x);
    assert!(app.workspace_bar.more.right() <= header.right());
    assert!(app.workspace_bar.add.right() < app.workspace_bar.more.x);

    // 顶栏不再有工作区按钮：文件标签栏没有被压缩，也没有被挤到左栏的卡片里。
    assert!(app.layout.tabs.x >= app.layout.top_bar.x);
    assert!(app.workspace_bar.view.right() <= app.layout.sidebar.right());

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 读取窗口坐标 (`x`, `y`) 处的像素颜色（RGB 三元组）。
fn pixel(app: &App, (x, y): (f32, f32)) -> (u8, u8, u8) {
    let image = app.frame.renderer.as_ref().unwrap().image().unwrap();
    let at = (y * SCALE) as usize * image.width as usize + (x * SCALE) as usize;
    let px = image.pixels[at];
    (
        ((px >> 16) & 0xff) as u8,
        ((px >> 8) & 0xff) as u8,
        (px & 0xff) as u8,
    )
}

/// 关闭按钮：指针停在标签上才出现，鼠标路过时不出现。
#[test]
fn the_close_button_shows_after_the_pointer_rests_on_the_tab() {
    let a = search_folder("workspace-hover-a");
    let b = search_folder("workspace-hover-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    paint(&mut app);

    let chip = app.workspace_bar.chips[1];
    let at = (
        chip.close.x + chip.close.w / 2.0,
        chip.close.y + chip.close.h / 2.0,
    );
    let before = pixel(&app, at);

    // 鼠标刚移到标签上：关闭按钮还没有出现。
    app.on_mouse_move(chip.rect.x + chip.rect.w / 2.0, chip.rect.y + 4.0);
    paint(&mut app);
    assert!(
        !app.workspace_bar.close_ready,
        "刚移上去时关闭按钮还没有出现"
    );
    assert_eq!(pixel(&app, at), before, "关闭按钮还没有出现");

    // 停留够久：右端出现关闭按钮。
    app.workspace_bar.rest = Some(std::time::Instant::now() - crate::theme::CLOSE_HOVER_DELAY);
    app.tick();
    paint(&mut app);
    assert!(app.workspace_bar.close_ready);
    assert_ne!(before, pixel(&app, at), "停留后标签右端应有关闭按钮");

    // 鼠标落在关闭按钮上时，按钮不能消失。
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
    paint(&mut app);
    assert_ne!(before, pixel(&app, at), "鼠标落在关闭按钮上时它不能消失");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 鼠标从标签上划过时关闭按钮不出现：那一段仍属于标签，点它切换工作区。
#[test]
fn the_close_button_does_not_appear_while_the_mouse_passes_over() {
    let a = search_folder("wbp");
    let b = search_folder("wbp2");
    let mut app = app_with(&a, false);
    // 侧栏宽一些，两个标签都在可见范围内：第一个标签的关闭按钮点得到。
    app.panels.sidebar.width = 400.0;
    open_workspace(&mut app, &b);
    paint(&mut app);

    let chip = app.workspace_bar.chips[0];
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
    assert_eq!(app.pointer.hover, Hover::Workspace(chip.index));
    assert!(!app.workspace_bar.close_ready);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 标签上的关闭按钮只关掉这一个工作区：关掉非当前的那个，当前工作区不受影响。
#[test]
fn the_close_button_on_a_workspace_tab_closes_only_that_workspace() {
    let a = search_folder("wbc");
    let b = search_folder("wbc2");
    let mut app = app_with(&a, false);
    // 侧栏宽一些，两个标签都在可见范围内：第一个标签的关闭按钮点得到。
    app.panels.sidebar.width = 400.0;
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 悬停到标签上时，关闭按钮可以命中。
    let chip = app.workspace_bar.chips[0];
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
    rest_on_chip(&mut app);
    assert_eq!(app.pointer.hover, Hover::WorkspaceClose(chip.index));

    // 点它只关掉这个工作区。
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(root(&app), winc_config::files::canonicalize(&b).unwrap());

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 关掉当前的工作区时切到相邻的一个。
#[test]
fn closing_the_current_workspace_switches_to_the_neighbour() {
    let a = search_folder("workspace-close-current-a");
    let b = search_folder("workspace-close-current-b");
    let mut app = app_with(&a, false);
    let root_a = root(&app);
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 当前工作区是第二个，关掉它之后切回第一个。
    let chip = app.workspace_bar.chips[1];
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
    rest_on_chip(&mut app);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);
    assert_eq!(
        root(&app),
        winc_config::files::canonicalize(&root_a).unwrap()
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 关掉工作区后提示条上可以撤销：项目重新打开，位置回到关闭前的地方。
#[test]
fn a_closed_workspace_can_be_restored_from_the_notice() {
    let a = search_folder("workspace-undo-a");
    let b = search_folder("workspace-undo-b");
    let c = search_folder("workspace-undo-c");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    open_workspace(&mut app, &c);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 3);

    // 关掉中间那个工作区。
    app.close_workspace(1);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 2);
    assert_eq!(root(&app), winc_config::files::canonicalize(&c).unwrap());

    // 提示条上有「撤销」，点它重新打开那个项目。
    let action = app.notice.action.clone().expect("提示条上有撤销");
    assert!(matches!(action, NoticeAction::UndoCloseWorkspace { .. }));
    app.run_notice_action();
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 3);
    assert_eq!(app.workspaces.active, 1, "重新打开的是刚关掉的那个");
    assert_eq!(root(&app), winc_config::files::canonicalize(&b).unwrap());
    // 位置回到关闭前的地方：它在第二个。
    assert_eq!(
        app.workspaces.slots[1].folder,
        winc_config::files::canonicalize(&b).unwrap()
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&c);
}

/// 拖动工作区标签可以调整顺序：被拖的工作区跟着当前项走。
#[test]
fn dragging_a_workspace_tab_reorders_the_workspaces() {
    // 名字短，两个标签都能放进左栏顶部那一行。
    let a = search_folder("wa");
    let b = search_folder("wb");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    paint(&mut app);

    let name_a = app.workspaces.slots[0].name.clone();
    let name_b = app.workspaces.slots[1].name.clone();
    let chips = app.workspace_bar.chips.clone();
    assert!(chips[0].rect.x >= 0.0, "两个标签都要在可见范围内");

    // 按住第一个标签往右拖，越过第二个标签的中线。
    app.on_mouse_move(chips[0].rect.x + 4.0, chips[0].rect.y + 4.0);
    app.on_mouse_down(MouseButton::Left);
    assert!(app.tabs.drag.is_none(), "与文件标签的拖动互不干扰");
    app.on_mouse_move(
        chips[1].rect.x + chips[1].rect.w / 2.0 + 8.0,
        chips[0].rect.y + 4.0,
    );
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);

    // 顺序换过来，当前工作区跟着被拖的那个走。
    assert_eq!(app.workspaces.slots[0].name, name_b);
    assert_eq!(app.workspaces.slots[1].name, name_a);
    assert_eq!(app.workspaces.active, 1);
    assert_eq!(root(&app), winc_config::files::canonicalize(&a).unwrap());

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 拖动越过两个以上的标签时，动的是被拖的那个工作区。
#[test]
fn dragging_a_workspace_tab_past_two_neighbours_moves_the_dragged_one() {
    let a = search_folder("ka");
    let b = search_folder("kb");
    let c = search_folder("kc");
    let mut app = app_with(&a, false);
    // 侧栏宽一些，三个标签都放得下，拖动时标签条不会滚动。
    app.panels.sidebar.width = 400.0;
    open_workspace(&mut app, &b);
    open_workspace(&mut app, &c);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 3);

    let chips = app.workspace_bar.chips.clone();
    // 按住第一个标签，先拖过第二个的中线，再拖过第三个的中线。
    app.on_mouse_move(chips[0].rect.x + 4.0, chips[0].rect.y + 4.0);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_move(
        chips[1].rect.x + chips[1].rect.w / 2.0 + 8.0,
        chips[0].rect.y + 4.0,
    );
    app.on_mouse_move(
        chips[2].rect.x + chips[2].rect.w / 2.0 + 8.0,
        chips[0].rect.y + 4.0,
    );
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);

    // 被拖的工作区到了末尾，其余两个次序不变，当前工作区跟着它走。
    assert_eq!(app.workspaces.slots[0].name, "kb");
    assert_eq!(app.workspaces.slots[1].name, "kc");
    assert_eq!(app.workspaces.slots[2].name, "ka");
    assert_eq!(app.workspaces.active, 2);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&c);
}

/// 标签放不下时横向滚动，当前工作区的标签始终可见。
#[test]
fn the_workspace_tabs_scroll_to_keep_the_current_one_visible() {
    let a = search_folder("workspace-scroll-a");
    let b = search_folder("workspace-scroll-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    // 名字长到标签条放不下。
    app.workspaces.slots[0].name = "一个很长的项目名字-用来把标签条撑开".to_owned();
    app.workspaces.slots[1].name = "另一个很长的项目名字-也放不下".to_owned();
    paint(&mut app);

    // 标签条滚动过，当前工作区的标签在可见范围内。
    assert!(app.workspace_bar.scroll > 0.0);
    let active = app.workspaces.active;
    let chip = app
        .workspace_bar
        .chips
        .iter()
        .find(|c| c.index == active)
        .unwrap();
    let view = app.workspace_bar.view;
    assert!(chip.rect.x >= view.x - 1.0 && chip.rect.right() <= view.right() + 1.0);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 左栏收起时标签条跟着收起（切换仍可用快捷键）。
#[test]
fn the_workspace_tabs_hide_with_the_left_column() {
    let a = search_folder("workspace-hidden-a");
    let b = search_folder("workspace-hidden-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    paint(&mut app);

    app.toggle_sidebar();
    paint(&mut app);
    for chip in &app.workspace_bar.chips {
        assert!(
            chip.rect.right() <= app.layout.sidebar.right(),
            "左栏收起后标签还在可见范围内"
        );
    }

    // 快捷键仍然能切换工作区。
    app.run_action(Action::NextWorkspace);
    paint(&mut app);
    assert_eq!(app.workspaces.active, 0);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 辅助进程是窗口共用的：打开另一个项目（面板开着）时，那边也有终端。
///
/// 辅助进程只握手一次，握手的状态跟着窗口走，不跟着工作区走：否则新工作区里的面板永远
/// 等不到握手，终端建不出来，面板是空的。
#[test]
fn a_new_workspace_gets_its_terminal_when_the_helper_is_already_running() {
    let a = search_folder("workspace-terminal-a");
    let b = search_folder("workspace-terminal-b");
    let mut app = app_with(&a, false);
    app.config.terminal_enabled = Some(true);
    // 辅助进程已经跑着、握过手了。
    app.attach_idle_helper();
    // 第一个项目里开着终端面板。
    app.toggle_terminal();
    app.relayout();
    app.sync_terminal();
    assert_eq!(app.terminal.tabs.len(), 1, "第一个项目里有终端");

    // 打开第二个项目：面板跟着开，里面也有终端（辅助进程是共用的，不用重新握手）。
    open_workspace(&mut app, &b);
    app.relayout();
    app.sync_terminal();
    assert_eq!(app.terminal.tabs.len(), 1, "第二个项目里也有终端");

    // 切回第一个项目：它的终端还在。
    app.switch_workspace(0);
    app.relayout();
    app.sync_terminal();
    assert_eq!(app.terminal.tabs.len(), 1, "切回去终端还在");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 关掉一个项目里的最后一个终端：辅助进程是共用的，别处还挂着终端，它不能断。
#[test]
fn closing_a_workspace_terminal_keeps_the_shared_helper_for_the_others() {
    let a = search_folder("workspace-shared-a");
    let b = search_folder("workspace-shared-b");
    let mut app = app_with(&a, false);
    app.config.terminal_enabled = Some(true);
    app.attach_idle_helper();
    app.toggle_terminal();
    app.relayout();
    app.sync_terminal();
    assert_eq!(app.terminal.tabs.len(), 1, "第一个项目里有终端");

    open_workspace(&mut app, &b);
    app.relayout();
    app.sync_terminal();
    assert_eq!(app.terminal.tabs.len(), 1, "第二个项目里也有终端");

    // 关掉第二个项目里的终端：第一个项目的终端还挂在同一个辅助进程上，它不能断。
    app.close_terminal_pane();
    assert_eq!(app.terminal.tabs.len(), 0, "第二个项目里的终端关掉了");
    assert!(app.terminal_connected(), "辅助进程还连着");

    // 切回第一个项目：它的终端还在。
    app.switch_workspace(0);
    app.relayout();
    assert_eq!(app.terminal.tabs.len(), 1, "第一个项目的终端还在");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 起了名字的工作区：切走再切回来，标签上还是新名字。
#[test]
fn a_renamed_workspace_keeps_its_name_in_the_tab() {
    let a = search_folder("workspace-rename-a");
    let b = search_folder("workspace-rename-b");
    let mut app = app_with(&a, false);
    app.rename_group("新名字");
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 切回去：标签上还是起过的名字。
    app.switch_workspace(0);
    paint(&mut app);
    assert_eq!(app.workspace_name(0), "新名字");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 退出时窗口里开着的项目都记下来，下次启动时都还在。
#[test]
fn the_workspaces_are_remembered_and_reopened() {
    let a = search_folder("workspace-keep-a");
    let b = search_folder("workspace-keep-b");
    let mut app = app_with(&a, false);
    // 两个项目里各打开一个文件。
    app.open_path(&a.join("src/main.rs"));
    open_workspace(&mut app, &b);
    app.open_path(&b.join("src/config.rs"));
    paint(&mut app);

    app.save_session();
    let state = app.state.clone();
    assert_eq!(state.workspaces.folders.len(), 2, "两个项目都记下了");
    assert_eq!(state.workspaces.active, 1, "记下了当时打开的是哪一个");

    // 下次启动：窗口里开着的项目都还在，各自的标签也恢复。
    let mut next = blank_app(false);
    next.state = state;
    let workspaces = next.state.workspaces.clone();
    next.restore_workspaces(&workspaces);
    paint(&mut next);
    assert_eq!(next.workspaces.slots.len(), 2);
    assert_eq!(next.workspaces.active, 1);
    assert_eq!(root(&next), winc_config::files::canonicalize(&b).unwrap());
    assert_eq!(names(&next), ["config.rs"]);

    // 切回第一个项目：它的标签也还在。
    next.switch_workspace(0);
    paint(&mut next);
    assert_eq!(root(&next), winc_config::files::canonicalize(&a).unwrap());
    assert_eq!(names(&next), ["main.rs"]);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 单工作区模式：退出时不记工作区列表，下次启动按原来那样只开一个项目。
#[test]
fn the_workspaces_are_not_remembered_in_the_single_workspace_mode() {
    let a = search_folder("workspace-single-a");
    let b = search_folder("workspace-single-b");
    let mut app = app_with(&a, false);
    app.config.multi_workspace = Some(false);
    app.open_path(&b);
    paint(&mut app);
    assert_eq!(app.workspaces.slots.len(), 1);

    app.save_session();
    assert_eq!(app.state.workspaces.folders.len(), 1);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 标签条放不下时横向滚动：滚出去的项目点不到，滚过来才能点。
#[test]
fn the_workspace_tabs_scroll_horizontally() {
    let a = search_folder("workspace-scroll2-a");
    let b = search_folder("workspace-scroll2-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    // 名字长到标签条放不下。
    app.workspaces.slots[0].name = "一个很长的项目名字-用来把标签条撑开".to_owned();
    app.workspaces.slots[1].name = "另一个很长的项目名字-也放不下".to_owned();
    paint(&mut app);

    let view = app.workspace_bar.view;
    assert!(app.workspace_bar.content > view.w, "标签条放不下");
    assert!(
        app.workspace_bar.list.is_some(),
        "右端有列出全部工作区的「⌄」"
    );
    // 当前工作区的标签在可见范围内。
    let active = app.workspaces.active;
    let chip = app
        .workspace_bar
        .chips
        .iter()
        .find(|c| c.index == active)
        .unwrap();
    assert!(chip.rect.right() <= view.right() + 1.0);

    // 滚轮落在标签条上时横向滚动它，不滚文件树。
    app.on_mouse_move(chip.rect.x + 4.0, chip.rect.y + 4.0);
    let before = app.workspace_bar.scroll;
    app.on_wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
    assert!(
        app.workspace_bar.scroll < before,
        "滚轮向下滚时标签条往左滚"
    );
    assert_eq!(app.panels.tree.scroll, 0.0, "文件树没有跟着滚");

    // 滚到最左端：第一个工作区的标签看得见也点得到。
    app.workspace_bar.scroll = 0.0;
    paint(&mut app);
    let first = app.workspace_bar.chips[0];
    app.on_mouse_move(first.rect.x + 4.0, first.rect.y + 4.0);
    assert_eq!(app.pointer.hover, Hover::Workspace(0));

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 标签条放不下时，滚出去的项目既看不见也点不到。
#[test]
fn the_workspace_tabs_outside_the_view_are_not_clickable() {
    let a = search_folder("workspace-outside-a");
    let b = search_folder("workspace-outside-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    app.workspaces.slots[0].name = "一个很长的项目名字-用来把标签条撑开".to_owned();
    app.workspaces.slots[1].name = "另一个很长的项目名字-也放不下".to_owned();
    paint(&mut app);

    // 第一个工作区的标签有一部分滚出了可见范围：那一段点不到。
    let first = app.workspace_bar.chips[0];
    let outside = first.rect.x + 2.0;
    assert!(
        outside < app.workspace_bar.view.x,
        "标签的一部分在可见范围之外"
    );
    app.on_mouse_move(outside, first.rect.y + 4.0);
    assert_eq!(app.pointer.hover, Hover::None, "看不见的标签点不到");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 标签条放不下时，右端的「⌄」列出全部工作区，滚出去的项目也能切到。
#[test]
fn the_workspace_list_menu_lists_all_the_workspaces() {
    let a = search_folder("workspace-menu-a");
    let b = search_folder("workspace-menu-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    app.workspaces.slots[0].name = "一个很长的项目名字-用来把标签条撑开".to_owned();
    app.workspaces.slots[1].name = "另一个很长的项目名字-也放不下".to_owned();
    paint(&mut app);

    let list = app.workspace_bar.list.expect("有列出全部工作区的「⌄」");
    app.on_mouse_move(list.x + list.w / 2.0, list.y + list.h / 2.0);
    assert_eq!(app.pointer.hover, Hover::WorkspaceList);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    assert_eq!(
        app.overlays.menu_button,
        Some(crate::app::MenuButton::WorkspaceList)
    );
    let entries = &app
        .overlays
        .context_menu
        .as_ref()
        .expect("弹出了工作区列表")
        .entries;
    assert_eq!(entries.len(), 2);
    for i in 0..2 {
        assert_eq!(
            app.menu_checked(MenuCommand::ShowWorkspace(i)),
            i == app.workspaces.active
        );
        let label = app.menu_label(MenuCommand::ShowWorkspace(i));
        assert_eq!(label.title, app.workspace_name(i));
    }

    // 选第一项：切到那个工作区。
    app.run_menu_command(MenuCommand::ShowWorkspace(0));
    paint(&mut app);
    assert_eq!(app.workspaces.active, 0);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
