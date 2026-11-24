//! 截图：三栏布局、多工作区的顶栏和「面板」设置页。

use super::*;

use winc_config::Panel;

/// 在 `dir` 里建一个仓库；没有 `git` 时返回 `false`。
fn git_init(dir: &Path) -> bool {
    #[allow(clippy::disallowed_methods, reason = "测试中直接运行本机的 git")]
    let status = std::process::Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir)
        .status();
    status.is_ok_and(|status| status.success())
}

/// 截图：左栏是文件树，右栏是 Git 的提交视图（`shot-three-columns-*`），以及右栏里放着
/// 文件树和搜索面板的样子。
#[test]
#[ignore]
fn shoot_the_three_columns() {
    let dir = search_folder("shot-columns");
    std::fs::write(dir.join("src/other.rs"), "fn main() {}\n").unwrap();
    if !git_init(&dir) {
        return;
    }

    let mut app = app_with(&dir, false);
    app.settings_toggle(Setting::GitModule);
    app.git_discover();
    app.git_switch_repository(0);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    app.open_path(&dir.join("src/main.rs"));
    paint(&mut app);
    shoot(&mut app, "shot-three-columns");

    // 鼠标停在右栏的开关按钮上：按钮有悬停效果。
    let button = app.layout.right_button;
    app.on_mouse_move(button.x + button.w / 2.0, button.y + button.h / 2.0);
    paint(&mut app);
    shoot(&mut app, "shot-three-columns-hover");

    // 提交视图挪到左栏，右栏里是文件树和搜索面板。
    app.config.panels_left = Some(vec![Panel::Commit]);
    app.config.panels_right = Some(vec![Panel::Files, Panel::Search]);
    app.adopt_panels();
    app.run_action(crate::keymap::Action::FindInFiles);
    app.edit_field(|f| f.insert("config"));
    app.run_global_search();
    finish_search(&mut app);
    paint(&mut app);
    shoot(&mut app, "shot-three-columns-search");

    let _ = std::fs::remove_dir_all(&dir);
}

/// 截图：左栏顶部有两个工作区时的标签条（`shot-workspaces-*`）。
#[test]
#[ignore]
fn shoot_the_workspace_bar() {
    let a = search_folder("shot-ws-a");
    let b = search_folder("shot-ws-b");
    let mut app = app_with(&a, false);
    app.open_path(&b);
    // 第二个项目在 WSL 发行版中：标签上写项目名，位置用小号淡色字标出。
    app.workspaces.slots[1].host = Some(winc_editor::host::Host::Wsl {
        distro: "Ubuntu".to_owned(),
        root: false,
    });
    paint(&mut app);
    shoot(&mut app, "shot-workspaces");

    // 鼠标停在标签上：右端出现关闭按钮。
    let chip = app.workspace_bar.chips[1];
    app.on_mouse_move(chip.rect.x + chip.rect.w / 2.0, chip.rect.y + 4.0);
    paint(&mut app);
    shoot(&mut app, "shot-workspaces-hover");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 截图：左栏很窄、工作区标签放不下时的样子（`shot-workspace-overflow`）。
#[test]
#[ignore]
fn shoot_the_workspace_bar_overflow() {
    let a = search_folder("shot-ws-over-a");
    let b = search_folder("shot-ws-over-b");
    let c = search_folder("shot-ws-over-c");
    let mut app = app_with(&a, false);
    app.panels.sidebar.width = 260.0;
    app.open_path(&b);
    app.open_path(&c);
    // 标签上写的是格子里记下的项目名：设长一些，看标签条放不下时的样子。
    app.workspaces.slots[0].label = "一个很长的项目名字-用来把标签条撑开".to_owned();
    app.workspaces.slots[1].label = "另一个很长的项目名字-也放不下".to_owned();
    app.workspaces.slots[2].label = "第三个也很长-标签条放不下".to_owned();
    paint(&mut app);
    shoot(&mut app, "shot-workspace-overflow");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&c);
}

/// 截图：设置里的「面板」页（`shot-panels-page-*`）。
#[test]
#[ignore]
fn shoot_the_panels_page() {
    let dir = search_folder("shot-panels-page");
    let mut app = app_with(&dir, false);
    app.run_action(crate::keymap::Action::Settings);
    app.settings_show_page(SettingsPage::Panels);
    paint(&mut app);
    shoot(&mut app, "shot-panels-page");

    let _ = std::fs::remove_dir_all(&dir);
}

/// 截图：关掉一个工作区后提示条上的「撤销」（`shot-workspace-undo`）。
#[test]
#[ignore]
fn shoot_the_workspace_undo_notice() {
    let a = search_folder("shot-ws-undo-a");
    let b = search_folder("shot-ws-undo-b");
    let mut app = app_with(&a, false);
    app.open_path(&b);
    paint(&mut app);
    app.close_workspace(0);
    paint(&mut app);
    shoot(&mut app, "shot-workspace-undo");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 截图：右栏收起、顶栏右端由应用绘制时的样子（`shot-right-column-closed`）。
#[test]
#[ignore]
fn shoot_the_right_column_closed() {
    let dir = sample_folder("shot-right-closed");
    std::fs::write(dir.join("doc.md"), "# 标题\n\n正文。\n").unwrap();
    let mut app = app_with(&dir, false);
    // 应用自己绘制标题栏（Windows 与 Linux 上如此）。
    app.window.controls = true;
    app.settings_toggle(Setting::GitModule);
    app.config.panels_left = Some(vec![Panel::Files]);
    app.config.panels_right = Some(vec![Panel::Commit]);
    app.adopt_panels();
    app.open_path(&dir.join("doc.md"));
    app.toggle_right_column();
    paint(&mut app);
    shoot(&mut app, "shot-right-column-closed");
    let _ = std::fs::remove_dir_all(&dir);
}
