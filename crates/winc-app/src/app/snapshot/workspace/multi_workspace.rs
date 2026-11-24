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
