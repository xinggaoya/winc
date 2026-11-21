//! 从 GitHub 的发布里取更新：版本号、下载地址和校验值。
//!
//! GitHub 为每个仓库提供发布的订阅源（`releases.atom`），里面是最近的发布，从新到旧。
//! 附件的地址由仓库、标签和文件名拼出（`releases/download/<标签>/<文件名>`）；校验值取自
//! 发布里的 `SHA256SUMS`，它与附件一起上传。

use roxmltree::Document;

/// 校验值文件的名字：发布里与附件一起上传的校验和清单。
pub const CHECKSUMS: &str = "SHA256SUMS";

/// 发布的订阅源：仓库的最近发布。
pub fn feed(repository: &str) -> String {
    format!("https://github.com/{repository}/releases.atom")
}

/// 发布里某个附件的下载地址。
pub fn download(repository: &str, tag: &str, name: &str) -> String {
    format!("https://github.com/{repository}/releases/download/{tag}/{name}")
}

/// 订阅源里的发布标签，从新到旧。
pub fn tags(xml: &str) -> Vec<String> {
    let Ok(doc) = Document::parse(xml) else {
        return Vec::new();
    };
    doc.descendants()
        .filter(|node| node.has_tag_name("entry"))
        .filter_map(|entry| {
            entry
                .children()
                .find(|child| child.has_tag_name("title"))
                .and_then(|child| child.text())
                .map(str::trim)
                .filter(|title| title.starts_with('v'))
                .map(str::to_owned)
        })
        .collect()
}

/// 校验和清单里某个文件的校验值。
///
/// 清单是 GNU `sha256sum` 的格式：校验值、两个空格、文件名。
pub fn checksum(sums: &str, name: &str) -> Option<[u8; 32]> {
    sums.lines().find_map(|line| {
        let (digest, file) = line.split_once("  ")?;
        let file = file.trim();
        let file = file.strip_prefix('*').unwrap_or(file);
        if file != name {
            return None;
        }
        hex(digest.trim())
    })
}

/// 十六进制的校验值转成字节。
fn hex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (n, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(text.get(n * 2..n * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}
