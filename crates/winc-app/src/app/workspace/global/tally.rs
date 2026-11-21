//! 跨文件替换的统计：替换了多少处、哪些文件失败。

use std::path::Path;

use winc_i18n::t;

/// 跨文件替换的结果统计。
#[derive(Default)]
pub(crate) struct ReplaceTally {
    /// 有替换的文件数。
    pub(crate) files: usize,
    /// 替换的总处数。
    pub(crate) count: usize,
    /// 有替换且已打开的文件数：替换在文档中进行，尚未保存。
    pub(crate) opened: usize,
    /// 替换失败的文件，每项是文件名和原因（显示给用户的文字）。
    pub(crate) failed: Vec<String>,
}

impl ReplaceTally {
    /// 记录文件 `path` 的替换结果：成功时为替换的处数，失败时为原因；`open` 表示文件已打开。
    pub(crate) fn add(&mut self, path: &Path, result: Result<usize, String>, open: bool) {
        match result {
            Ok(n) => {
                self.files += usize::from(n > 0);
                self.count += n;
                self.opened += usize::from(open && n > 0);
            }
            Err(e) => {
                let name = winc_editor::display_name(path);
                let message = t!("workspace-replace-failed-file", name = name, error = e);
                self.failed.push(message);
            }
        }
    }
}
