//! 多工作区模式的测试：同一个窗口里同时打开多个项目，顶栏里切换，各自的标签、文件树和
//! 终端都留着。

use super::*;

use crate::keymap::Action;

/// 打开目录 `path`：多工作区模式下它成了窗口里的另一个工作区。
fn open_workspace(app: &mut App, path: &Path) {
    app.open_path(path);
}

/// 当前工作区打开的目录。
fn root(app: &App) -> PathBuf {
    app.editor.workspace.as_ref().unwrap().root.clone()
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

/// 标签悬停时右端出现关闭按钮。
#[test]
fn the_close_button_shows_on_hover() {
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

    // 悬停标签：右端出现关闭按钮。
    app.on_mouse_move(chip.rect.x + chip.rect.w / 2.0, chip.rect.y + 4.0);
    paint(&mut app);
    assert_ne!(before, pixel(&app, at), "悬停时标签右端应有关闭按钮");

    // 鼠标落在关闭按钮上时，按钮不能消失。
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
    paint(&mut app);
    assert_ne!(before, pixel(&app, at), "鼠标落在关闭按钮上时它不能消失");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 标签上的关闭按钮只关掉这一个工作区：关掉非当前的那个，当前工作区不受影响。
#[test]
fn the_close_button_on_a_workspace_tab_closes_only_that_workspace() {
    let a = search_folder("workspace-close-a");
    let b = search_folder("workspace-close-b");
    let mut app = app_with(&a, false);
    open_workspace(&mut app, &b);
    paint(&mut app);

    // 悬停到标签上时，关闭按钮可以命中。
    let chip = app.workspace_bar.chips[0];
    app.on_mouse_move(chip.close.x + 4.0, chip.close.y + 4.0);
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
