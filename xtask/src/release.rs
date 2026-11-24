//! 打包脚本文件的读取：测试读 release/ 下的安装包脚本。

use std::path::{Path, PathBuf};

/// 读 release/ 下的文件，路径相对仓库根目录。
pub(crate) fn release_file(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let path: PathBuf = root.join("release").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
