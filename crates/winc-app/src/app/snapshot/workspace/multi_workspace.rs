//! 多工作区下的未保存内容：切换时写备份、后台保存的结果、关闭工作区前的检查。

use super::*;

use super::session::backup_root;

/// 切走一个工作区时，它没写盘的备份当场写完：未保存的内容不会只存在于内存里。
#[test]
fn switching_workspaces_writes_the_pending_backups() {
    let a = search_folder("ws-write-a");
    let b = search_folder("ws-write-b");
    let root = backup_root("ws-write");
    let mut app = app_with(&a.join("src/main.rs"), false);
    app.backup.root = Some(root.clone());
    app.run(Command::Insert("// 改了，还没到写备份的时间\n".into()));
    assert!(app.written_backups().is_empty(), "还没写备份：切走时才写");

    // 切到第二个工作区：第一个工作区那份改动当场写成备份。
    app.open_path(&b);
    let files = app.written_backups();
    assert_eq!(files.len(), 1, "切换时把没写盘的备份写完");
    assert!(files[0].exists(), "备份写出来了：{}", files[0].display());

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    let _ = std::fs::remove_dir_all(&root);
}

/// 后台保存完成时文档在别的工作区：保存结果照样收下，切回来不再是未保存状态。
#[test]
fn a_background_save_finishes_in_the_workspace_that_owns_the_document() {
    let dir = sample_folder("ws-background-save");
    let dir = winc_config::files::canonicalize(&dir).unwrap();
    let path = dir.join("big.log");
    let line = "2026-11-24 ok\n";
    let count = winc_editor::LARGE_FILE as usize / line.len() + 10;
    std::fs::write(&path, line.repeat(count)).unwrap();
    let other = search_folder("ws-background-other");

    let mut app = app_with(&dir, false);
    app.open_path(&path);
    let index = app.editor.active.unwrap();
    let id = app.editor.documents[index].id;
    app.run(Command::Insert("A".into()));

    // 后台保存：先取出要写的内容。
    let job = {
        let doc = app.editor.document_mut(id).unwrap();
        doc.begin_save().unwrap().unwrap()
    };

    // 保存还在后台时切到另一个工作区。
    app.open_path(&other);
    assert_eq!(app.workspaces.active, 1, "切到了第二个工作区");

    // 后台线程写完，结果这时才送来：文档在另一个工作区里。
    let saved = job.run();
    app.finish_background_save(id, saved);

    // 切回去：那个文档不再是未保存状态。
    app.switch_workspace(0);
    let doc = app.editor.document(id).unwrap();
    assert!(
        !doc.is_dirty(),
        "保存的结果要送到文档所在的工作区，切回来不该还显示未保存"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&other);
}

/// 关闭有未保存修改的工作区：先询问，放弃修改才关掉。
#[test]
fn closing_a_workspace_asks_about_unsaved_changes() {
    let a = search_folder("ws-close-a");
    let b = search_folder("ws-close-b");
    let mut app = app_with(&a.join("src/main.rs"), false);
    app.run(Command::Insert("// 没保存的改动\n".into()));
    app.open_path(&b);
    assert_eq!(app.workspaces.slots.len(), 2);

    // 关掉有未保存修改的工作区：先询问。
    app.close_workspace(0);
    assert!(app.closing.prompt.is_some(), "关掉之前先问未保存的修改");
    assert_eq!(app.workspaces.slots.len(), 2, "还没有关掉");

    // 回答「不保存」：工作区关掉了。
    app.answer_save_prompt(SaveChoice::Discard);
    assert_eq!(app.workspaces.slots.len(), 1, "关掉了那个工作区");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 取消关闭工作区：它还在，刚才切过去的工作区也切回去。
#[test]
fn cancelling_the_close_of_a_workspace_keeps_it() {
    let a = search_folder("ws-cancel-a");
    let b = search_folder("ws-cancel-b");
    let mut app = app_with(&a.join("src/main.rs"), false);
    app.run(Command::Insert("// 没保存的改动\n".into()));
    app.open_path(&b);
    let before = app.workspaces.active;

    app.close_workspace(0);
    assert!(app.closing.prompt.is_some());
    app.answer_save_prompt(SaveChoice::Cancel);
    assert_eq!(app.workspaces.slots.len(), 2, "取消以后工作区还在");
    assert_eq!(app.workspaces.active, before, "切回原来的工作区");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 关闭工作区时它的终端标签也一并关掉：辅助进程里不再留着它的 shell。
#[test]
fn closing_a_workspace_closes_its_terminal() {
    let a = search_folder("ws-term-a");
    let b = search_folder("ws-term-b");
    let mut app = app_with(&a, false);
    // 给第一个工作区开一个终端标签（本地仿真，不启动辅助进程）。
    let size = app.terminal_size();
    app.terminal
        .tabs
        .push(crate::app::terminal::TermTab::new(7, size));
    let tab = app.terminal.tabs[0].id;
    app.open_path(&b);

    // 关掉第一个工作区：它的终端标签不再留在窗口里。
    app.close_workspace(0);
    assert_eq!(app.workspaces.slots.len(), 1);
    let mut kept = app.terminal.tabs.iter().chain(
        app.workspaces
            .slots
            .iter()
            .flat_map(|s| s.terminal.tabs.iter()),
    );
    assert!(
        kept.all(|t| t.id != tab),
        "关闭工作区时它的终端标签也关掉了"
    );

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 关闭工作区时，辅助进程里的 shell 也结束：不再留着它的终端。
#[cfg(unix)]
#[test]
fn closing_a_workspace_tells_the_helper_to_end_its_shell() {
    let a = search_folder("ws-term-helper-a");
    let b = search_folder("ws-term-helper-b");
    let mut app = app_with(&a, false);
    super::super::terminal::show_terminal(&mut app, "$ ");
    let tab = app.terminal.tabs[0].id;
    let log = app.attach_recording_helper(&a);
    app.open_path(&b);

    app.close_workspace(0);
    let closed = app
        .sent_to_helper(&log)
        .into_iter()
        .filter(|m| matches!(m, winc_pty::protocol::ToHelper::Close { tab: t } if *t == tab))
        .count();
    assert_eq!(closed, 1, "关闭工作区时它的终端标签也通知了辅助进程");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 切换工作区时，那个工作区自己的状态留着：文件树还在原来的位置。
#[test]
fn switching_workspaces_keeps_their_own_state() {
    let a = search_folder("ws-state-a");
    let b = search_folder("ws-state-b");
    let mut app = app_with(&a, false);
    // 第一个工作区的文件树滚到下面。
    app.panels.tree.scroll = 42.0;
    app.open_path(&b);
    assert_eq!(app.panels.tree.scroll, 0.0, "第二个工作区还没有滚动");

    // 切回第一个：它的滚动位置还在。
    app.switch_workspace(0);
    assert_eq!(app.panels.tree.scroll, 42.0, "切回来时文件树还在原来的位置");

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// 左栏没有面板时，工作区标签条照样画出来：工作区要能切换。
#[test]
fn the_workspace_bar_is_drawn_without_panels() {
    let a = search_folder("ws-nopanel-a");
    let mut app = app_with(&a, false);
    // 把文件面板挪到右栏：左栏没有面板了。
    app.config.panels_left = Some(Vec::new());
    app.config.panels_right = Some(vec![winc_config::Panel::Files]);
    app.adopt_panels();
    paint(&mut app);
    assert_eq!(app.layout.left_count, 0, "左栏没有面板");

    let chip = app.workspace_bar.chips[0].clone();
    let image = app.frame.renderer.as_ref().unwrap().image().unwrap();
    let scale = image.width as f32 / app.layout.window.w;
    let x = ((chip.rect.x + 2.0) * scale) as usize;
    let y = ((chip.rect.y + chip.rect.h / 2.0) * scale) as usize;
    let pixel = image.pixels[y * image.width as usize + x];
    // 标签左边是卡片本身的底色：两处不同说明标签那块实底画出来了。
    let outside = image.pixels[y * image.width as usize + ((chip.rect.x - 4.0) * scale) as usize];
    assert_ne!(pixel, outside, "标签条画出来了");

    let _ = std::fs::remove_dir_all(&a);
}

/// 右栏收起与展开有过渡：动画中途只露出一部分。
#[test]
fn the_right_column_slides_in_and_out() {
    use std::time::Duration;

    let dir = search_folder("right-column-motion");
    let mut app = app_with(&dir, false);
    // 右栏里放文件树（提交视图要目录是仓库才显示）。
    app.config.panels_left = Some(Vec::new());
    app.config.panels_right = Some(vec![winc_config::Panel::Files]);
    app.adopt_panels();
    assert_eq!(app.right_shown(), 1.0, "右栏开着");

    // 收起：动画中途右栏只露出一部分。
    app.panels.right_hidden = true;
    app.panels.right_motion = Some(crate::app::workspace::columns::RightMotion {
        from: 1.0,
        clock: Clock::frozen(
            Duration::from_millis(100),
            crate::app::animation::SIDEBAR_ANIMATION,
        ),
    });
    let shown = app.right_shown();
    assert!(
        shown > 0.0 && shown < 1.0,
        "动画中途右栏只露出一部分：{shown}"
    );

    // 动画结束：右栏完全收起来。
    app.panels.right_motion = Some(crate::app::workspace::columns::RightMotion {
        from: 1.0,
        clock: Clock::frozen(
            Duration::from_millis(200),
            crate::app::animation::SIDEBAR_ANIMATION,
        ),
    });
    assert_eq!(app.right_shown(), 0.0, "动画结束时右栏收起来了");

    let _ = std::fs::remove_dir_all(&dir);
}
