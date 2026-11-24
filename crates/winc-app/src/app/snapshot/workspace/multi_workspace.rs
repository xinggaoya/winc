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
    assert!(
        app.written_backups().is_empty(),
        "还没写备份：切走时才写"
    );

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
