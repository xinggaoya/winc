//! 三栏布局的测试：左栏、中间那栏（编辑区和终端，固定）和右栏；面板放在哪一栏、各栏里的
//! 顺序、右栏的宽度，以及「面板」设置页里的调整。

use super::*;

use crate::keymap::Action;
use winc_config::Panel;

/// 在 `dir` 里初始化一个仓库；没有 `git` 时返回 `false`。
fn git_init(dir: &Path) -> bool {
    #[allow(clippy::disallowed_methods, reason = "测试中直接运行本机的 git")]
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status();
    status.is_ok_and(|status| status.success())
}

/// 打开设置页的「面板」页。
fn open_panels_page(app: &mut App) {
    app.run_action(Action::Settings);
    app.settings_show_page(crate::layout::SettingsPage::Panels);
    paint(app);
}

/// 「面板」页里某个按钮的中心点。
fn button_point(app: &App, button: crate::app::settings::panels::PanelButton) -> (f32, f32) {
    let layout = app.panels_layout();
    let row = |panel: Panel| {
        layout
            .left
            .iter()
            .chain(&layout.right)
            .chain(&layout.hidden)
            .find(|row| row.panel == panel)
            .copied()
            .unwrap()
    };
    match button {
        crate::app::settings::panels::PanelButton::Up(panel) => {
            let r = row(panel).up;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
        crate::app::settings::panels::PanelButton::Down(panel) => {
            let r = row(panel).down;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
        crate::app::settings::panels::PanelButton::Side(panel) => {
            let r = row(panel).side;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
        crate::app::settings::panels::PanelButton::Hide(panel) => {
            let r = row(panel).hide;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
        crate::app::settings::panels::PanelButton::WidthDown => {
            let r = layout.width_down;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
        crate::app::settings::panels::PanelButton::WidthUp => {
            let r = layout.width_up;
            (r.x + r.w / 2.0, r.y + r.h / 2.0)
        }
    }
}

/// 窗口分成三栏：左栏放文件树，中间是编辑区，右栏放 Git 的提交视图。
#[test]
fn the_window_has_a_left_a_middle_and_a_right_column() {
    let dir = sample_folder("columns");
    let mut app = app_with(&dir, false);
    paint(&mut app);

    // 左栏是文件树，右栏是提交视图（不在仓库里时提交视图不显示，右栏不占地方）。
    assert_eq!(app.layout.left_count, 1);
    assert_eq!(
        app.panel_slot(Panel::Files).unwrap().panel,
        Some(Panel::Files)
    );
    // 右栏没有面板时不占地方：整张卡片在窗口右边外面。
    assert!(app.layout.right.x >= app.layout.window.right());

    // 中间那栏是编辑区：它在左栏右边，右栏左边。
    let editor = app.layout.editor;
    assert!(editor.x >= app.layout.sidebar.right());
    assert!(editor.right() <= app.layout.window.right());

    // 目录在仓库里时右栏出现，提交视图在里面。
    if !git_init(&dir) {
        return;
    }
    app.settings_toggle(Setting::GitModule);
    app.git_discover();
    app.git_switch_repository(0);
    app.relayout();
    paint(&mut app);
    assert_eq!(app.layout.right_count, 1);
    assert_eq!(
        app.panel_slot(Panel::Commit).unwrap().panel,
        Some(Panel::Commit)
    );
    // 右栏在窗口右边，中间那栏在它左边。
    assert!(app.layout.right.x >= app.layout.editor.right());
    assert_eq!(app.layout.right.right(), app.layout.window.right() - 6.0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 面板放在哪一栏、各栏里的顺序由配置决定；配置改了，界面照着摆。
#[test]
fn the_config_decides_which_panels_are_in_which_column() {
    let dir = sample_folder("columns-config");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files, Panel::Commit]);
    app.config.panels_right = Some(vec![]);
    app.adopt_panels();
    paint(&mut app);

    // 左栏里从上到下是文件树和提交视图，右栏空着。
    assert_eq!(app.layout.left_count, 2);
    assert_eq!(
        app.panel_slot(Panel::Files).unwrap().panel,
        Some(Panel::Files)
    );
    assert_eq!(
        app.panel_slot(Panel::Commit).unwrap().panel,
        Some(Panel::Commit)
    );
    assert_eq!(app.layout.right_count, 0);

    // 两个面板各占一半高：文件树在上面，提交视图在下面。
    let files = app.panel_slot(Panel::Files).unwrap();
    let commit = app.panel_slot(Panel::Commit).unwrap();
    assert!(files.card.y < commit.card.y);
    assert_eq!(files.card.bottom() + 4.0, commit.card.y);

    // 换个配置：提交视图挪到右栏。
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    paint(&mut app);
    assert_eq!(app.layout.left_count, 1);
    assert_eq!(app.layout.right_count, 1);
    assert!(app.panel_slot(Panel::Commit).unwrap().card.x >= app.layout.right.x);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 全局搜索改用弹窗：配置里留着 `"search"` 时，两栏里也不出现搜索面板。
#[test]
fn the_search_panel_is_not_in_the_columns() {
    let dir = sample_folder("columns-no-search");
    let mut app = app_with(&dir, false);
    app.config.panels_left = Some(vec![Panel::Files, Panel::Search]);
    app.adopt_panels();
    paint(&mut app);

    assert_eq!(app.columns().0, vec![Panel::Files]);
    assert!(app.panel_slot(Panel::Search).is_none());
    assert_eq!(app.layout.left_count, 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 「面板」设置页：把面板移到另一栏、上下移动、藏起来，都写回配置。
#[test]
fn the_panels_page_moves_hides_and_reorders_panels() {
    let dir = sample_folder("columns-page");
    let mut app = app_with(&dir, false);
    open_panels_page(&mut app);

    // 默认：左栏是文件树，右栏是提交视图，没有藏起来的面板。
    let (left, right, hidden) = app.panel_groups();
    assert_eq!(left, vec![Panel::Files]);
    assert_eq!(right, vec![Panel::Commit]);
    assert!(hidden.is_empty());

    // 把文件树移到右栏：配置跟着变。
    use crate::app::settings::panels::PanelButton;
    let at = button_point(&app, PanelButton::Side(Panel::Files));
    click(&mut app, at);
    paint(&mut app);
    let (left, right, _) = app.panel_groups();
    assert!(!left.contains(&Panel::Files));
    assert!(right.contains(&Panel::Files));
    assert_eq!(app.config.panels_left, Some(left));
    assert_eq!(app.config.panels_right, Some(right));

    // 藏起来：不在任何一栏里。
    let at = button_point(&app, PanelButton::Hide(Panel::Files));
    click(&mut app, at);
    paint(&mut app);
    let (left, right, hidden) = app.panel_groups();
    assert!(!left.contains(&Panel::Files) && !right.contains(&Panel::Files));
    assert!(hidden.contains(&Panel::Files));

    // 再点一次放回右栏。
    let at = button_point(&app, PanelButton::Hide(Panel::Files));
    click(&mut app, at);
    paint(&mut app);
    let (_, right, _) = app.panel_groups();
    assert!(right.contains(&Panel::Files));

    let _ = std::fs::remove_dir_all(&dir);
}

/// 「面板」设置页里上下移动按钮：同一栏里的顺序跟着变。
#[test]
fn the_panels_page_reorders_the_panels_of_a_column() {
    let dir = sample_folder("columns-order");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files, Panel::Commit]);
    app.config.panels_right = Some(vec![]);
    app.adopt_panels();
    open_panels_page(&mut app);

    use crate::app::settings::panels::PanelButton;
    // 把提交视图上移一位：文件树和提交视图换了位置。
    let at = button_point(&app, PanelButton::Up(Panel::Commit));
    click(&mut app, at);
    paint(&mut app);
    let (left, _, _) = app.panel_groups();
    assert_eq!(left, vec![Panel::Commit, Panel::Files]);
    assert_eq!(app.config.panels_left, Some(left));

    // 再下移回去。
    let at = button_point(&app, PanelButton::Down(Panel::Commit));
    click(&mut app, at);
    paint(&mut app);
    let (left, _, _) = app.panel_groups();
    assert_eq!(left, vec![Panel::Files, Panel::Commit]);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 右栏的宽度：设置页里一档一档地调，配置和界面都跟着变。
#[test]
fn the_right_column_width_is_configurable() {
    let dir = sample_folder("columns-width");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    paint(&mut app);

    let before = app.panels.layout.right_width;
    open_panels_page(&mut app);
    use crate::app::settings::panels::PanelButton;
    let at = button_point(&app, PanelButton::WidthUp);
    click(&mut app, at);
    paint(&mut app);
    assert_eq!(app.panels.layout.right_width, before + 20.0);
    assert_eq!(app.config.panel_right_width, Some(before + 20.0));

    // 右栏变宽时中间那栏变窄。
    let editor = app.layout.editor;
    let at = button_point(&app, PanelButton::WidthDown);
    click(&mut app, at);
    paint(&mut app);
    assert_eq!(app.panels.layout.right_width, before);
    assert!(app.layout.editor.w > editor.w);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 拖动行首的把手：面板落到别的栏里。
#[test]
fn dragging_a_panel_row_moves_it_to_another_column() {
    let dir = sample_folder("columns-drag");
    let mut app = app_with(&dir, false);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    open_panels_page(&mut app);

    // 按住文件树那一行的把手，拖到右栏提交视图那一行上面，松手。
    let layout = app.panels_layout();
    let handle = layout
        .left
        .iter()
        .find(|row| row.panel == Panel::Files)
        .unwrap()
        .handle;
    let target = layout
        .right
        .iter()
        .find(|row| row.panel == Panel::Commit)
        .unwrap()
        .row;
    app.on_mouse_move(handle.x + 4.0, handle.y + handle.h / 2.0);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_move(target.x + 40.0, target.y + target.h / 2.0);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);

    let (left, right, _) = app.panel_groups();
    assert!(!left.contains(&Panel::Files));
    assert!(right.contains(&Panel::Files));

    let _ = std::fs::remove_dir_all(&dir);
}

/// 右栏可以收起：收起时中间那栏占满窗口，右栏不占地方。
#[test]
fn the_right_column_can_be_hidden() {
    let dir = sample_folder("columns-hide");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    paint(&mut app);

    let wide = app.layout.editor.w;
    app.toggle_right_column();
    paint(&mut app);
    // 右栏没有面板时不占地方：整张卡片在窗口右边外面。
    assert!(app.layout.right.x >= app.layout.window.right());
    assert!(app.layout.editor.w > wide);
    app.toggle_right_column();
    paint(&mut app);
    assert_eq!(app.layout.editor.w, wide);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 每一栏里的面板都占满自己那一份，互不重叠。
#[test]
fn the_panels_of_a_column_do_not_overlap() {
    let dir = sample_folder("columns-slots");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files, Panel::Commit]);
    app.config.panels_right = Some(vec![]);
    app.adopt_panels();
    paint(&mut app);

    let files = app.panel_slot(Panel::Files).unwrap();
    let commit = app.panel_slot(Panel::Commit).unwrap();
    assert!(files.card.bottom() <= commit.card.y);
    assert!(files.body.bottom() <= files.card.bottom());
    assert!(commit.body.bottom() <= commit.card.bottom());
    // 面板的正文在它自己的卡片里。
    assert!(files.body.x >= files.card.x);
    assert!(files.body.right() <= files.card.right());

    let _ = std::fs::remove_dir_all(&dir);
}

/// 右栏里的面板可以点：悬停有反馈，点击有反应。
#[test]
fn the_panels_in_the_right_column_are_clickable() {
    let dir = sample_folder("columns-click");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    paint(&mut app);

    // 目录不是仓库：右栏里写明原因，并给出初始化和克隆的入口。
    let card = app.panel_card(Panel::Commit);
    assert!(card.w > 0.0 && card.h > 0.0);

    // 提交面板里的按钮可以点：目录不是仓库时给出「初始化仓库」和「克隆仓库」。
    use crate::app::git::commit::CommitTarget;
    let hits = app.hits.commit.clone();
    let (rect, _) = hits
        .iter()
        .find(|(_, target)| *target == CommitTarget::InitRepository)
        .expect("右栏的提交面板里没有「初始化仓库」按钮");
    app.on_mouse_move(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    assert_eq!(
        app.pointer.hover,
        Hover::Commit(CommitTarget::InitRepository),
        "右栏里的按钮没有悬停效果"
    );

    // 顶栏右端有右栏的开关按钮。
    let button = app.layout.right_button;
    assert!(button.w > 0.0);
    app.on_mouse_move(button.x + button.w / 2.0, button.y + button.h / 2.0);
    assert_eq!(app.pointer.hover, Hover::RightButton);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    // 收起后右栏不占地方，按钮还在。
    assert!(app.layout.right.x >= app.layout.window.right());
    assert!(app.layout.right_button.w > 0.0);
    app.on_mouse_down(MouseButton::Left);
    app.on_mouse_up(MouseButton::Left);
    paint(&mut app);
    assert!(app.layout.right.x < app.layout.window.right());

    let _ = std::fs::remove_dir_all(&dir);
}

/// 顶栏由系统处理（Windows 上按标题栏）：其中的控件要登记为客户区，否则点击被当成拖动
/// 窗口，按钮点不动。
#[test]
fn the_controls_in_the_title_bar_are_client_area() {
    let dir = sample_folder("columns-caption");
    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    paint(&mut app);

    let client = app.caption_controls();
    let inside = |rect: Rect| {
        let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        client.iter().any(|r| r.contains(x, y))
    };

    // 右栏的开关按钮。
    assert!(inside(app.layout.right_button), "右栏开关不在客户区里");
    // 两个工作区时顶栏里的切换按钮。
    let other = sample_folder("columns-caption-2");
    app.open_path(&other);
    paint(&mut app);
    let client = app.caption_controls();
    let inside = |rect: Rect| {
        let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        client.iter().any(|r| r.contains(x, y))
    };
    for (rect, _) in &app.workspace_bar.chips {
        assert!(inside(*rect), "工作区按钮不在客户区里");
    }
    assert!(
        inside(app.workspace_bar.add),
        "「打开项目」按钮不在客户区里"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&other);
}
