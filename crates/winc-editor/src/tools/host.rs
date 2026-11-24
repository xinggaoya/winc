//! 路径所在的主机：本机、Windows 上的某个 WSL 发行版，或经 SSH 连接的远端主机。
//!
//! 在路径所在的主机上运行的程序（`git` 等）使用该主机上的路径写法，输出的也是该主机上的路径。
//! 本模块在两种写法之间换算，调用方不区分主机的种类。WSL 的换算见 [`crate::wsl`]，远端主机
//! 的换算见 `winc_backend`。

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// 把在 `root` 所在主机上运行的程序输出的路径 `text` 转换为本程序使用的路径。
///
/// 绝对路径按该主机上的路径处理；其余路径相对于 `root`。
#[must_use]
pub fn resolve(root: &Path, text: &str) -> PathBuf {
    match winc_backend::locate(root).filter(|_| text.starts_with('/')) {
        Some(remote) => winc_backend::virtual_path(&remote.target, text.as_bytes()),
        None => crate::wsl::resolve(root, text),
    }
}

/// 返回把 `path` 传给在 `root` 所在主机上运行的程序时使用的写法。
///
/// 两者不在同一主机上时无法换算，返回 `path` 原样的文本；程序找不到该路径并报错。
#[must_use]
pub fn arg(root: &Path, path: &Path) -> String {
    match (winc_backend::locate(root), winc_backend::locate(path)) {
        (Some(root), Some(remote)) if root.target == remote.target => {
            String::from_utf8_lossy(&remote.path).into_owned()
        }
        _ => crate::wsl::git_arg(root, path),
    }
}

/// 路径所在的主机：Windows 上的某个 WSL 发行版，或经 SSH 连接的远端主机。
///
/// 本机上的路径不在任何主机上（[`host`] 返回 `None`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    /// WSL 发行版。
    Wsl {
        /// 发行版名称。
        distro: String,
        /// 目录是不是发行版里的根目录。
        root: bool,
    },
    /// 经 SSH 连接的远端主机。
    Ssh {
        /// 连接目标：主机名，有端口时其后是 `:端口`。
        target: String,
        /// 目录是不是主机上的根目录。
        root: bool,
    },
}

impl Host {
    /// 位置的写法：`WSL: Ubuntu` / `SSH: example.com`。
    pub fn full(&self) -> String {
        match self {
            Host::Wsl { distro, .. } => format!("WSL: {distro}"),
            Host::Ssh { target, .. } => format!("SSH: {target}"),
        }
    }

    /// 位置的简短名字：发行版名或主机名。
    pub fn short(&self) -> &str {
        match self {
            Host::Wsl { distro, .. } => distro,
            Host::Ssh { target, .. } => target,
        }
    }

    /// 目录是不是所在系统的根目录。
    pub fn is_root(&self) -> bool {
        match self {
            Host::Wsl { root, .. } | Host::Ssh { root, .. } => *root,
        }
    }
}

/// 返回 `path` 所在的主机；本机上的路径返回 `None`。
///
/// 发行版中的路径只在 Windows 上存在：其他平台上 `\\wsl.localhost\…` 不是发行版中的路径。
#[must_use]
pub fn host(path: &Path) -> Option<Host> {
    if let Some(at) = winc_backend::locate(path) {
        return Some(Host::Ssh {
            target: at.target.as_str().to_owned(),
            root: at.path == b"/",
        });
    }
    crate::wsl::locate_here(path).map(|place| Host::Wsl {
        distro: place.distro,
        root: place.path == "/",
    })
}
