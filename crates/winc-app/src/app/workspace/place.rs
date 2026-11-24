//! 左栏顶部工作区标签条的位置：每个工作区一个标签，末尾是「打开项目」与项目菜单两个按钮。
//!
//! 两个按钮钉在左栏的右端，标签在它们左边横向滚动；放不下时标签向左滚动，当前工作区的标签始终
//! 可见，右端浮着一个「⌄」按钮，列出全部工作区。

use super::*;

/// 标签里除文字以外的部分（徽章、关闭按钮与两侧留白，逻辑像素）。
pub(crate) const CHIP_CHROME: f32 = 6.0 + 20.0 + 5.0 + 4.0 + 16.0 + 6.0;
/// 标签的最小宽度（逻辑像素）。
pub(crate) const CHIP_MIN: f32 = 72.0;
/// 项目名最多占的宽度（逻辑像素），超出部分截短。
pub(crate) const CHIP_NAME_MAX: f32 = 140.0;
/// 位置标记最多占的宽度（逻辑像素），超出部分截短。
pub(crate) const CHIP_HOST_MAX: f32 = 48.0;
/// 项目名与位置标记之间的距离（逻辑像素）。
pub(crate) const CHIP_HOST_GAP: f32 = 6.0;
/// 项目名至少留的宽度（逻辑像素）：地方不够时位置标记先让位。
const NAME_MIN: f32 = 36.0;
/// 末尾两个按钮的宽度（逻辑像素）。
const END_W: f32 = 24.0;

/// 项目名与位置标记在 `room`（逻辑像素）里怎么排：返回名字可用的宽度与位置标记的宽度。
///
/// 位置标记放得下时占它的宽度，项目名用剩下的；放不下时舍位置标记，项目名占满。
pub(crate) fn split_chip_room(name_w: f32, host_w: f32, room: f32) -> (f32, Option<f32>) {
    let keep = name_w.min(NAME_MIN);
    if host_w > 0.0 && room >= host_w + CHIP_HOST_GAP + keep {
        (room - host_w - CHIP_HOST_GAP, Some(host_w))
    } else {
        (room, None)
    }
}

/// 标签上要画的两段文字。
#[derive(Debug, Clone, Default)]
pub(crate) struct ChipText {
    /// 项目名：按开头截断到放得下的宽度。
    pub name: String,
    /// 位置标记：发行版名或主机名；没有位置或放不下时没有。
    pub host: Option<String>,
}

/// 位置标记的文本与宽度：截到 [`CHIP_HOST_MAX`] 以内。
///
/// 标签留出的位置与画出来的文字是同一段：不会画到标签外面。
pub(crate) fn chip_host_text(
    text: &str,
    size: f32,
    measure: &mut impl FnMut(&str, f32) -> f32,
) -> (String, f32) {
    let text = crate::text::fit(text, CHIP_HOST_MAX, false, |s| measure(s, size)).into_owned();
    let width = measure(&text, size);
    (text, width)
}

/// 标签里的文字怎么排：项目名优先，位置标记放不下时舍去。
///
/// `room` 是两段文字合起来可用的宽度（逻辑像素）。项目名按开头截断；位置标记的文本截到
/// [`CHIP_HOST_MAX`] 以内，放不下时整段舍去。
pub(crate) fn chip_text(
    name: &str,
    host: Option<&str>,
    room: f32,
    size: f32,
    small: f32,
    measure: &mut impl FnMut(&str, f32) -> f32,
) -> ChipText {
    let (host_text, host_w) = match host {
        Some(text) => {
            let (text, width) = chip_host_text(text, small, measure);
            (Some(text), width)
        }
        None => (None, 0.0),
    };
    let name_w = measure(name, size).min(CHIP_NAME_MAX);
    let (name_room, host_room) = split_chip_room(name_w, host_w, room);
    let name = crate::text::fit(name, name_room, false, |s| measure(s, size)).into_owned();
    ChipText {
        name,
        host: host_room.map(|_| host_text.unwrap_or_default()),
    }
}
/// 标签与末尾按钮之间的间距（逻辑像素）。
const END_GAP: f32 = 4.0;
/// 标签里关闭按钮的边长（逻辑像素）。
const CLOSE: f32 = 16.0;
/// 放不下时浮在标签右端的「⌄」按钮的宽度（逻辑像素）。
const LIST_W: f32 = 20.0;
/// 「⌄」按钮与标签之间的距离（逻辑像素）。
const LIST_GAP: f32 = 6.0;

impl App {
    /// 排好左栏顶部工作区标签条的位置。
    ///
    /// 没有打开目录时只有末尾两个按钮；标签放不下时在可见范围内横向滚动，当前工作区的标签始终
    /// 可见，右端浮着一个「⌄」按钮，列出全部工作区。
    pub(in crate::app) fn place_workspaces(&mut self) {
        let header = self.layout.sidebar_header;
        let y = header.y + (header.h - TAB_HEIGHT) / 2.0;
        let right = header.right() - 8.0;
        let count = self.workspaces.slots.len();
        // 各工作区在标签上要写的两段文字：项目名与位置。
        let labels: Vec<WorkspaceLabel> = (0..count).map(|i| self.workspace_label(i)).collect();

        let bar = &mut self.workspace_bar;
        bar.chips.clear();
        bar.add = Rect::default();
        bar.more = Rect::default();
        bar.view = Rect::default();
        bar.list = None;
        bar.content = 0.0;

        // 末尾两个按钮：项目菜单在最右，「打开项目」在它左边。
        let more = Rect::new(right - END_W, y, END_W, TAB_HEIGHT);
        let add = Rect::new(more.x - END_GAP - END_W, y, END_W, TAB_HEIGHT);
        bar.more = more;
        bar.add = add;

        if count == 0 {
            return;
        }
        let Some(renderer) = self.frame.renderer.as_ref() else {
            return;
        };
        let font = self.fonts.ui;
        let small = self.fonts.ui_small;
        // 标签从左栏左边 8 像素处开始，滚到「打开项目」按钮左边为止。
        let left = header.x + 8.0;
        let room = (add.x - END_GAP - left).max(0.0);

        // 每个标签的宽度：徽章与关闭按钮占固定宽度，名字与位置标记按长度算，长的截短。
        let mut widths = Vec::with_capacity(count);
        for label in &labels {
            let name = renderer
                .measure(FontKind::Ui, font, &label.name)
                .min(CHIP_NAME_MAX);
            let host = match &label.host {
                Some(host) => {
                    let (_, width) = chip_host_text(host.short(), small, &mut |s, size| {
                        renderer.measure(FontKind::Ui, size, s)
                    });
                    width + CHIP_HOST_GAP
                }
                None => 0.0,
            };
            widths.push((CHIP_CHROME + name + host).ceil().max(CHIP_MIN));
        }
        let mut content: f32 = widths.iter().sum::<f32>() + TAB_GAP * (count as f32 - 1.0);

        // 放不下时右端留出「⌄」的位置：它浮在标签上，点它列出全部工作区，滚出去的项目也能切到。
        // 标签比可见范围还宽时收窄它：名字截短，徽章与关闭按钮都留着。
        let narrow = room - LIST_W - LIST_GAP;
        let mut list = None;
        let mut view = Rect::new(left, y, room, TAB_HEIGHT);
        if content > room {
            let limit = narrow.max(CHIP_MIN);
            for width in &mut widths {
                *width = width.min(limit);
            }
            content = widths.iter().sum::<f32>() + TAB_GAP * (count as f32 - 1.0);
            // 收窄后还是放不下：右端留出「⌄」的位置。
            if content > narrow {
                list = Some(Rect::new(left + room - LIST_W, y, LIST_W, TAB_HEIGHT));
                view = Rect::new(left, y, narrow.max(0.0), TAB_HEIGHT);
            }
        }

        let mut x = left;
        for (index, width) in widths.iter().enumerate() {
            let rect = Rect::new(x, y, *width, TAB_HEIGHT);
            let close = Rect::new(
                rect.right() - 6.0 - CLOSE,
                y + (TAB_HEIGHT - CLOSE) / 2.0,
                CLOSE,
                CLOSE,
            );
            // 标签里的文字：项目名按开头截断，位置标记放不下时舍去。
            let label = &labels[index];
            let name_x = rect.x + 6.0 + 20.0 + 5.0;
            let room = (close.x - name_x).max(0.0);
            let text = chip_text(
                &label.name,
                label.host.as_ref().map(|host| host.short()),
                room,
                font,
                small,
                &mut |s, size| renderer.measure(FontKind::Ui, size, s),
            );
            bar.chips.push(WorkspaceChip {
                rect,
                close,
                text,
                index,
            });
            x += width + TAB_GAP;
        }
        bar.view = view;
        bar.content = content;
        bar.list = list;

        // 当前工作区变了，或标签条的大小变了（侧栏宽度、项目名），就把它滚进可见范围：
        // 手动滚动不因此被打断。
        let active = self.workspaces.active;
        let changed = bar.fitted != (view.w, content);
        bar.fitted = (view.w, content);
        if (bar.revealed != Some(active) || changed)
            && let Some(rect) = bar.chip(active)
        {
            let left_edge = rect.x - bar.scroll;
            let right_edge = rect.right() - bar.scroll;
            // 滚到当前工作区：它在可见范围的哪边就往哪边滚。
            if right_edge > view.right() {
                bar.scroll = rect.right() - view.right();
            } else if left_edge < view.x {
                bar.scroll = rect.x - view.x;
            }
            bar.revealed = Some(active);
        }
        bar.scroll = bar.scroll.clamp(0.0, bar.max_scroll());
        for chip in &mut bar.chips {
            chip.rect.x -= bar.scroll;
            chip.close.x -= bar.scroll;
        }
        // 标签条的位置也记在布局里（上游的同名区域在顶栏，本仓库把它搬到了左栏）。
        self.layout.workspaces = bar.view;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 位置标记放得下时占它的宽度，项目名用剩下的；放不下时舍位置标记，项目名优先。
    #[test]
    fn the_project_name_keeps_its_room() {
        // 放得下：名字与位置标记各占各的。
        assert_eq!(split_chip_room(100.0, 40.0, 146.0), (100.0, Some(40.0)));
        // 差一点放不下：舍位置标记，名字占满。
        assert_eq!(split_chip_room(100.0, 40.0, 45.0), (45.0, None));
        // 名字很短时也留着位置标记。
        assert_eq!(split_chip_room(20.0, 40.0, 66.0), (20.0, Some(40.0)));
        // 没有位置标记时名字占满。
        assert_eq!(split_chip_room(100.0, 0.0, 80.0), (80.0, None));
    }

    /// 位置标记放不下时舍去，项目名按开头截断；位置标记本身截到上限，不会画出标签。
    #[test]
    fn the_chip_text_keeps_the_project_name() {
        // 每个字符宽 8 逻辑像素。
        let mut measure = |s: &str, _size: f32| s.chars().count() as f32 * 8.0;
        // 放得下：项目名与位置标记都在。
        let text = chip_text("coda", Some("Ubuntu"), 200.0, 13.0, 11.0, &mut measure);
        assert_eq!(text.name, "coda");
        assert_eq!(text.host.as_deref(), Some("Ubuntu"));

        // 位置太长：截到上限（48 逻辑像素），不会画到标签外面。
        let text = chip_text(
            "coda",
            Some("Ubuntu-26.04"),
            200.0,
            13.0,
            11.0,
            &mut measure,
        );
        let host = text.host.expect("位置标记");
        assert!(host.ends_with('…'), "{host}");
        assert!(host.chars().count() <= 6, "{host}");

        // 地方不够：舍去位置标记，项目名按开头截断。
        let text = chip_text(
            "a-very-long-project-name",
            Some("Ubuntu"),
            40.0,
            13.0,
            11.0,
            &mut measure,
        );
        assert!(text.host.is_none(), "{:?}", text.host);
        assert!(text.name.ends_with('…'), "{}", text.name);
        assert!(text.name.starts_with('a'), "{}", text.name);
    }
}
