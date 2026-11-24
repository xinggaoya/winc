//! Windows 安装包的桌面快捷方式的测试：两种安装包都在桌面创建 Winc 快捷方式，卸载时删除。

use super::release::release_file;

/// `installer.nsi` 里某个段落（如 `Section "Winc" SEC_MAIN`）的正文。
fn nsis_section(header: &str) -> String {
    let script = release_file("windows/installer.nsi");
    let (_, rest) = script.split_once(header).unwrap();
    let (body, _) = rest.split_once("SectionEnd").unwrap();
    body.to_owned()
}

/// `installer.wxs` 里某个目录（如 `DesktopFolder`）的正文。
fn wix_directory(id: &str) -> String {
    let source = release_file("windows/installer.wxs");
    let (_, rest) = source
        .split_once(&format!("<Directory Id=\"{id}\""))
        .unwrap();
    let (body, _) = rest.split_once("</Directory>").unwrap();
    body.to_owned()
}

/// `installer.wxs` 里某个特性（如 `Main`）的正文。
fn wix_feature(id: &str) -> String {
    let source = release_file("windows/installer.wxs");
    let (_, rest) = source.split_once(&format!("<Feature Id=\"{id}\"")).unwrap();
    let (body, _) = rest.split_once("</Feature>").unwrap();
    body.to_owned()
}

#[test]
fn both_windows_installers_create_a_desktop_shortcut() {
    // NSIS：当前用户的桌面，快捷方式指向安装目录里的程序，用程序自己的图标。
    let main = nsis_section("Section \"Winc\" SEC_MAIN");
    assert!(
        main.contains(
            r#"CreateShortcut "$DESKTOP\Winc.lnk" "$INSTDIR\winc.exe" "" "$INSTDIR\winc.ico""#
        ),
        "NSIS 没有在桌面创建快捷方式",
    );
    // 桌面上显示的是快捷方式的文件名：旧的同名小写快捷方式要先删掉，否则显示的仍是小写。
    assert!(
        main.contains(r#"Delete "$DESKTOP\winc.lnk""#),
        "NSIS 没有删掉旧的同名快捷方式",
    );
    // MSI：整机安装，快捷方式放在所有用户的桌面上，名字与 NSIS 的相同。
    let desktop = wix_directory("DesktopFolder");
    assert!(
        desktop.contains(r#"<Shortcut Id="DesktopShortcutLink" Name="Winc""#),
        "MSI 没有在桌面创建快捷方式",
    );
    assert!(
        desktop.contains(r#"Target="[#WincExe]""#),
        "MSI 的桌面快捷方式没有指向程序",
    );
    assert!(
        desktop.contains(r#"Icon="WincIcon""#),
        "MSI 的桌面快捷方式没有图标",
    );
    // 组件要属于 Main 特性，否则不会随安装装上。
    assert!(
        wix_feature("Main").contains(r#"<ComponentRef Id="DesktopShortcut"/>"#),
        "MSI 没有安装桌面快捷方式",
    );
}

#[test]
fn both_windows_installers_remove_the_desktop_shortcut_on_uninstall() {
    // NSIS：卸载时删掉桌面的快捷方式。
    assert!(
        nsis_section("Section \"Uninstall\"").contains(r#"Delete "$DESKTOP\Winc.lnk""#),
        "NSIS 卸载时没有删掉桌面快捷方式",
    );
    // MSI：Windows Installer 随组件删掉它建的快捷方式。
    // 桌面是系统文件夹，卸载时不能删它本身。
    assert!(
        !wix_directory("DesktopFolder").contains("<RemoveFolder"),
        "不要删掉系统文件夹",
    );
}
