//! 路径所在的主机的判断。

use std::path::Path;

use super::*;

/// 本机上的路径不在任何主机上。
#[test]
fn a_local_path_is_on_no_host() {
    assert_eq!(host(Path::new("/home/me/project")), None);
}

/// 远端主机上的路径：主机名，以及它是不是主机上的根目录。
#[test]
fn a_remote_path_is_on_its_host() {
    let target = winc_backend::Target::new("example.com").unwrap();
    let path = winc_backend::virtual_path(&target, b"/home/me/project");
    assert_eq!(
        host(&path),
        Some(Host::Ssh {
            target: "example.com".to_owned(),
            root: false
        })
    );
    // 主机上的根目录。
    let root = winc_backend::virtual_path(&target, b"/");
    assert_eq!(
        host(&root),
        Some(Host::Ssh {
            target: "example.com".to_owned(),
            root: true
        })
    );
    // 带端口的目标：端口是目标的一部分，写出来要带上。
    let with_port = winc_backend::Target::new("example.com:2222").unwrap();
    let path = winc_backend::virtual_path(&with_port, b"/home/me/project");
    assert_eq!(
        host(&path),
        Some(Host::Ssh {
            target: "example.com:2222".to_owned(),
            root: false
        })
    );
}

/// WSL 发行版中的路径：发行版名称。这种写法只在 Windows 上表示发行版中的路径。
#[test]
fn a_wsl_path_is_on_its_distro() {
    let path = Path::new(r"\\wsl.localhost\Ubuntu-26.04\home\me\project");
    if cfg!(windows) {
        assert_eq!(
            host(path),
            Some(Host::Wsl {
                distro: "Ubuntu-26.04".to_owned(),
                root: false
            })
        );
    } else {
        assert_eq!(host(path), None);
    }
}
