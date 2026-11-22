//! 测试用的「辅助进程」替身：跨平台。
//!
//! 终端模块的用例要在没有真的辅助进程的情况下建出标签：接上一个什么也不做的程序，
//! 主程序发去的消息它不处理，标签照样当作已经建出来了。

use super::*;

/// 什么也不做的「辅助进程」：Unix 上是 `/usr/bin/true`，Windows 上是一个等着退出的批处理文件。
///
/// 两者都不读标准输入：主程序发去的消息不会被当成命令执行。
fn idle_program() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/usr/bin/true")
    }
    #[cfg(windows)]
    {
        let path =
            std::env::temp_dir().join(format!("winc-idle-helper-{}.bat", std::process::id()));
        std::fs::write(&path, "@echo off\r\nping -n 5 127.0.0.1 >nul\r\n")
            .expect("写入空闲辅助进程的批处理文件");
        path
    }
}

impl App {
    /// 测试用：接上一个什么也不做的「辅助进程」，发出去的消息不处理，标签当作已经可以用了。
    pub(crate) fn attach_idle_helper(&mut self) {
        self.terminal_client = Client::start(&idle_program(), || {}).ok();
        self.helper_ready = true;
    }
}
