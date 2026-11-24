//! 文本的测量与截短：按字素截断，emoji 不会被切开。
//!
//! 截短要按实际宽度来：绘制时量的是 [`winc_render::Frame`]，界面排布时量的是
//! [`winc_render::Renderer`]，两者给出的宽度相同。量法由调用方给出，两处都用同一段
//! 文字时结果也就一样。

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

/// 把文本截短到 `max`（逻辑像素）宽以内并接上「…」。
///
/// `middle` 为真时从中间截短（开头比结尾多留一个字素），否则从末尾截短。截短按字素进行
/// （带肤色修饰或组合字符的 emoji、带声调的字母各算一个字素），不会切开字素。每个字素只测量
/// 一次宽度，结果只拼接一次。从中间截短时至少截掉一个字素，避免原文完整却多出「…」。
///
/// `measure` 量一段文字的宽度。
pub(crate) fn fit<'a>(
    text: &'a str,
    max: f32,
    middle: bool,
    mut measure: impl FnMut(&str) -> f32,
) -> Cow<'a, str> {
    if text.is_empty() || measure(text) <= max {
        return Cow::Borrowed(text);
    }
    // 每个字素在 `text` 里的字节偏移和宽度。
    let chars: Vec<(usize, f32)> = text
        .grapheme_indices(true)
        .map(|(i, g)| (i, measure(g)))
        .collect();
    let n = chars.len();
    // 保留 `keep` 个字素时，开头留 `keep - keep / 2` 个、结尾留 `keep / 2` 个；`keep` 每增加 1，
    // 偶数时开头多一个，奇数时结尾多一个，宽度随 `keep` 单调不减。
    let added = |keep: usize| match (middle, keep % 2) {
        (true, 1) => chars[n - 1 - keep / 2].1,
        (true, _) => chars[keep / 2].1,
        (false, _) => chars[keep].1,
    };
    let limit = if middle { n - 1 } else { n };
    let (mut keep, mut width) = (0, measure("…"));
    while keep < limit && width + added(keep) <= max {
        width += added(keep);
        keep += 1;
    }
    let byte = |i: usize| chars.get(i).map_or(text.len(), |c| c.0);
    let (head, tail) = if middle {
        (keep - keep / 2, keep / 2)
    } else {
        (keep, 0)
    };
    // 「…」前面不留空白（截在两个词之间时开头会以空格结尾）。
    Cow::Owned(format!(
        "{}…{}",
        text[..byte(head)].trim_end(),
        &text[byte(n - tail)..]
    ))
}
