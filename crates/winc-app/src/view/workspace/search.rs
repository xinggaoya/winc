//! 全局搜索弹窗的绘制：条件区（查找、替换、包含、排除）和结果树。
//!
//! 弹窗是一张浮在编辑区上方的卡片：标题行连同工具按钮在最上面，下面依次是查找行、
//! 替换行、包含与排除行、结果统计行和结果列表。

use std::f32::consts::FRAC_PI_2;

use winc_i18n::t;
use winc_render::{Frame, Rect};

use super::*;
use crate::app::{Focus, ResultRow};
use crate::layout::{
    PANEL_CHEVRON, PANEL_LEFT, PANEL_TEXT, PanelButton, PanelParts, PanelTool, RowAction,
    row_actions,
};
use crate::view::controls::over;
use winc_config::Panel;

mod overflow;

use overflow::{actions_lag, draw_text, file_text, moved_back, sync_name, text_width};
pub(in crate::view) use overflow::{
    draw_result_overflow, draw_result_overflow_closing, result_overflow,
};

/// 结果行上悬停按钮左侧渐隐区域的宽度（逻辑像素，经验值）。
///
/// 其下的文本在该区域内逐渐淡出，而不是被直接截断。
const COVER_FADE: f32 = 20.0;

/// 绘制全局搜索弹窗，返回有焦点的输入框的光标矩形（供输入法候选框定位）。
pub(in crate::view) fn draw_search_dialog(app: &mut App, frame: &mut Frame<'_>) -> Option<Rect> {
    let card = app.layout.search_card;
    if card.w <= 0.0 || card.h <= 0.0 {
        return None;
    }

    // 卡片底色：浮层用浮层的底色，不透出下方的编辑区。
    draw_card(frame, app.palette, card, RADIUS_BAR, app.palette.popup_bg);

    app.panels.global.sync_rows();
    // 查找、替换输入框随内容长高：先按框的宽度折好行（宽度和高度无关），框的高度按行数定，
    // 绘制时继续使用折好的行。
    let wraps = {
        let parts = app.panel_parts();
        let width = PanelParts::text_area(parts.query, parts.case_toggle.x).w;
        let (query_focused, replace_focused) = (
            app.focus == Focus::GlobalQuery,
            app.focus == Focus::GlobalReplace,
        );
        let mut query = std::mem::take(&mut app.panels.global.query);
        let mut replace = std::mem::take(&mut app.panels.global.replace);
        let wraps = (
            wrap_area_field(app, frame, width, &mut query, query_focused),
            wrap_area_field(app, frame, width, &mut replace, replace_focused),
        );
        app.panels.global.query = query;
        app.panels.global.replace = replace;
        wraps
    };
    app.set_search_field_lines((wraps.0.lines(), wraps.1.lines()));
    let parts = app.panel_parts();
    draw_panel_header(app, frame, &parts);

    // 绘制输入框时需要更新其中记录的几何信息：先将输入框从 `app` 中取出，绘制后放回。
    let mut fields = [
        std::mem::take(&mut app.panels.global.query),
        std::mem::take(&mut app.panels.global.replace),
        std::mem::take(&mut app.panels.global.include),
        std::mem::take(&mut app.panels.global.exclude),
    ];
    let caret = draw_panel_inputs(app, frame, &parts, &mut fields, wraps);
    let [query, replace, include, exclude] = fields;
    app.panels.global.query = query;
    app.panels.global.replace = replace;
    app.panels.global.include = include;
    app.panels.global.exclude = exclude;

    draw_summary(app, frame, &parts);
    draw_results(app, frame, &parts);

    caret
}

/// 绘制面板第一行：左边是「搜索」，右端是三个按钮。
fn draw_panel_header(app: &App, frame: &mut Frame<'_>, parts: &PanelParts) {
    let p = app.palette;
    let header = parts.header;
    columns::draw_panel_title(app, frame, header, Panel::Search);

    let has_results = !app.panels.global.engine.results.is_empty();

    for (rect, tool) in parts.tools {
        // 没有结果时「清除」「全部折叠」置灰。
        let enabled = tool == PanelTool::Refresh || has_results;
        let hovered = enabled && app.pointer.hover == Hover::Panel(PanelButton::Tool(tool));
        draw_button(frame, p, rect, header, hovered, false);

        let color = match (enabled, hovered) {
            (false, _) => p.text_dim,
            (true, true) => p.text,
            (true, false) => p.text_muted,
        };
        let c = center(rect);

        match tool {
            PanelTool::Refresh => icons::refresh(frame, c, color, header),
            PanelTool::Clear => icons::clear_results(frame, c, color, header),
            PanelTool::CollapseAll if app.panels.global.all_collapsed() => {
                icons::expand_all(frame, c, color, header);
            }
            PanelTool::CollapseAll => icons::collapse_all(frame, c, color, header),
        }
    }
}

/// 弹窗里的输入框，见 [`field_box`]。
fn draw_field_box(app: &App, frame: &mut Frame<'_>, rect: Rect, focused: bool, error: bool) {
    field_box(app, frame, rect, app.palette.popup_bg, focused, error);
}

/// 绘制查找、替换、包含、排除四个输入框及其旁边的按钮，返回有焦点的输入框的光标矩形。
fn draw_panel_inputs(
    app: &App,
    frame: &mut Frame<'_>,
    parts: &PanelParts,
    [query, replace, include, exclude]: &mut [TextField; 4],
    (query_wrap, replace_wrap): (AreaWrap, AreaWrap),
) -> Option<Rect> {
    let p = app.palette;
    let g = &app.panels.global;
    let panel = app.panel_rect();
    let hovered = |button| app.pointer.hover == Hover::Panel(button);
    let error = g.engine.error().is_some() && g.due.is_none();

    let focused = app.focus == Focus::GlobalQuery;
    draw_field_box(app, frame, parts.query, focused, error);

    // 输入框左端是展开替换的箭头。
    draw_button(
        frame,
        p,
        parts.expand,
        panel,
        hovered(PanelButton::Expand),
        false,
    );
    // 箭头按替换行展开的比例从朝右旋转到朝下。
    let turned = g.replace_share() * FRAC_PI_2;
    icons::chevron_turned(
        frame,
        center(parts.expand),
        6.5,
        turned,
        p.text_muted,
        panel,
    );

    // 过渡途中文字按过渡结束后的高度排版，只绘制在当前的框内。
    let (query_h, replace_h) = g.settled_field_heights();
    let lines = |field: Rect, right: f32, height: f32| {
        let area = PanelParts::text_area(field, right);
        // 文字行在第一行的高度上居中。
        let pad = (crate::layout::PANEL_FIELD - AREA_LINE) / 2.0;
        Rect::new(area.x, field.y + pad, area.w, height - 2.0 * pad)
    };
    let area = lines(parts.query, parts.case_toggle.x, query_h);
    let mut caret = frame.within(parts.query, |frame| {
        draw_area_field(
            app,
            frame,
            area,
            query,
            query_wrap,
            t!("workspace-search-placeholder"),
            focused,
        )
    });

    let toggles = [
        (
            parts.case_toggle,
            g.case_sensitive,
            hovered(PanelButton::Case),
        ),
        (parts.word_toggle, g.whole_word, hovered(PanelButton::Word)),
        (parts.regex_toggle, g.regex, hovered(PanelButton::Regex)),
    ];
    draw_search_options(app, frame, toggles, panel);

    // 输入框最右端是「过滤」：展开了包含、排除，或者包含、排除不为空时用强调色。
    let button = parts.details;
    let active = g.details_open
        || g.include_ignored
        || !g.include.text().is_empty()
        || !g.exclude.text().is_empty();
    draw_button(
        frame,
        p,
        button,
        panel,
        hovered(PanelButton::Details),
        g.details_open,
    );
    let color = if active { p.accent } else { p.text_muted };
    icons::filter(frame, center(button), color, panel);

    // 展开、收起途中只绘制露出的部分（`replace_band`），并按展开比例淡入淡出。
    if parts.replace.h > 0.0 {
        let c = frame.within(reveal(parts.replace_band), |frame| {
            frame.with_opacity(g.replace_share(), |frame| {
                let focused = app.focus == Focus::GlobalReplace;
                draw_field_box(app, frame, parts.replace, focused, false);
                let area = lines(parts.replace, parts.replace_all.x, replace_h);
                let c = frame.within(parts.replace, |frame| {
                    draw_area_field(
                        app,
                        frame,
                        area,
                        replace,
                        replace_wrap,
                        t!("workspace-search-replace-placeholder"),
                        focused,
                    )
                });

                // 全部替换：图标按钮，与上一行的正则开关在水平方向上对齐；没有结果时置灰。
                let button = parts.replace_all;
                let enabled = g.engine.total() > 0;

                draw_button(
                    frame,
                    p,
                    button,
                    panel,
                    enabled && hovered(PanelButton::ReplaceAll),
                    false,
                );

                let color = if enabled { p.text_muted } else { p.text_dim };
                icons::replace_all(frame, center(button), color, panel);
                c
            })
        });
        caret = caret.or(c);
    }

    // 包含、排除：输入框内左侧是淡色的标签，其后才是输入区，标签不另占一行。
    if parts.include.h > 0.0 {
        let fields = [
            (
                parts.include,
                include,
                Focus::GlobalInclude,
                t!("workspace-search-include"),
                t!("workspace-search-include-example"),
            ),
            (
                parts.exclude,
                exclude,
                Focus::GlobalExclude,
                t!("workspace-search-exclude"),
                t!("workspace-search-exclude-example"),
            ),
        ];
        let glob_error = error && g.engine.is_filter_error();

        let c = frame.within(reveal(parts.details_band), |frame| {
            frame.with_opacity(g.details_share(), |frame| {
                let mut caret = None;
                for (rect, field, focus, title, placeholder) in fields {
                    let focused = app.focus == focus;
                    let error = glob_error && !field.text().is_empty();
                    draw_field_box(app, frame, rect, focused, error);
                    let title_x = rect.x - 8.0 + PANEL_LEFT;
                    let end = ui_text_small(frame, app, title_x, rect, title, p.text_muted, rect);
                    // 「排除」的右端让给「也搜索被忽略的文件」开关。
                    let right = if focus == Focus::GlobalExclude {
                        parts.ignored_toggle.x - 2.0
                    } else {
                        rect.right() - 6.0
                    };
                    let area = Rect::new(end + 8.0, rect.y, (right - end - 8.0).max(0.0), rect.h);
                    let c = draw_bar_field(app, frame, area, field, placeholder, focused);
                    caret = caret.or(c);
                }
                let button = parts.ignored_toggle;
                let on = g.include_ignored;
                draw_button(frame, p, button, panel, hovered(PanelButton::Ignored), on);
                let color = if on { p.accent } else { p.text_muted };
                icons::ignored_files(frame, center(button), color, panel);
                caret
            })
        });
        caret = caret.or(c);
    }

    caret
}

/// 返回展开、收起途中输入框可绘制的范围：露出的部分 `band`，下沿多留 1 像素。
///
/// 焦点和出错的外圈由 `field_box` 绘制在输入框外 1 像素处；不留出这 1 像素时，
/// 完全展开后最下面一个输入框外圈的底边会被裁掉。
fn reveal(band: Rect) -> Rect {
    Rect::new(band.x, band.y, band.w, band.h + 1.0)
}

/// 绘制结果统计行：显示「3 个文件中有 12 个结果」等统计，或搜索中、没有结果、出错的状态。
///
/// 没有内容时这一行不占空间（`parts.summary` 高度为 0），不绘制。
fn draw_summary(app: &App, frame: &mut Frame<'_>, parts: &PanelParts) {
    if parts.summary.h <= 0.0 {
        return;
    }
    let p = app.palette;
    let g = &app.panels.global;
    let engine = &g.engine;
    let row = parts.summary;
    let files = engine.results.len();
    let total = engine.total();
    let searching = engine.is_running() || g.due.is_some();

    let (text, color) = if let Some(error) = engine.error().filter(|_| g.due.is_none()) {
        (error.to_owned(), p.error)
    } else if g.query.text().is_empty() {
        (String::new(), p.text_dim)
    } else if searching && total == 0 {
        (t!("workspace-search-searching").to_owned(), p.text_dim)
    } else {
        // 截断了，或有行因正则回溯过多而放弃查找：计数是下限。
        let more = if engine.is_truncated() || engine.gave_up() {
            "+"
        } else {
            ""
        };
        let text = if searching {
            t!(
                "workspace-search-summary-searching",
                files = files,
                total = total,
                more = more
            )
        } else {
            t!(
                "workspace-search-summary",
                files = files,
                total = total,
                more = more
            )
        };
        (text, p.text_muted)
    };

    // 文本与输入框里的文字对齐。
    let x = row.x + PANEL_LEFT;
    let max = (parts.query.right() - x).max(0.0);
    let text = ellipsize(frame, &text, app.fonts.ui_small, max);
    ui_text_small(frame, app, x, row, &text, color, row);
}

/// 绘制结果列表：每个文件一行（箭头、图标、文件名、所在目录、匹配数），其下每处匹配一行。
fn draw_results(app: &mut App, frame: &mut Frame<'_>, parts: &PanelParts) {
    let area = parts.results;
    if app.no_search_results() {
        draw_empty_state(
            app,
            frame,
            (area, area),
            t!("workspace-search-no-results"),
            Some(if app.panels.global.include_ignored {
                t!("workspace-search-no-results-hint")
            } else {
                t!("workspace-search-no-results-hint-ignored")
            }),
        );
        return;
    }
    let animate = app.animate_tree_labels();
    app.panels.global.names.begin_frame(animate);
    let scroll = drawn_scroll(app.panels.global.scroll);
    let rows = app.panels.global.rows().len();
    let replace_open = app.panels.global.replace_open;
    // 行号一栏按结果里最大的行号定宽（各文件的匹配按行号排序，最后一处就是最大的），滚动、
    // 悬停时都不变。
    let last_line = app
        .panels
        .global
        .engine
        .results
        .iter()
        .filter_map(|f| f.matches.last())
        .map(|m| m.line + 1)
        .max()
        .unwrap_or(1);
    let digit = frame.measure(FontKind::Ui, app.fonts.ui_small, "0");
    let cx = ResultContext {
        area,
        selected: app
            .panels
            .global
            .selected_row()
            .map(|i| app.panels.global.rows()[i]),
        focused: app.focus == Focus::Results,
        replace_open,
        replacement: replace_open && app.panels.global.preview().is_some(),
        digit,
        numbers: digit * last_line.to_string().len() as f32,
    };

    // 文件正在折叠或展开时按动画中的结果行绘制：匹配行位置不变，只显示已露出的部分，
    // 其下方的行随之上下移动。动画时长很短，期间不显示悬停。
    let shared = &*app;
    if let Some(motion) = &shared.panels.global.motion {
        let (block, hidden) = (motion.block(), motion.hidden());
        for (v, &kind) in motion.rows.iter().enumerate() {
            let (row, clip) = crate::layout::tree_row_slot(area, scroll, v, Some(block), hidden);
            let clip = clip.intersect(area);
            if clip.h <= 0.0 || row.bottom() < area.y || row.y > area.bottom() {
                continue;
            }
            frame.within(clip, |frame| {
                draw_result_row(shared, frame, &cx, None, kind, row)
            });
        }
    } else {
        let first = (scroll / TREE_ROW).floor().max(0.0) as usize;
        let visible = (area.h / TREE_ROW).ceil() as usize + 1;
        for index in first..(first + visible).min(rows) {
            let row = shared.result_row_rect(index);
            let kind = shared.panels.global.rows()[index];
            draw_result_row(shared, frame, &cx, Some(index), kind, row);
        }
    }

    app.panels.global.names.end_frame();

    let content = (rows as f32 + 1.0) * TREE_ROW;
    draw_scrollbar(app, frame, ScrollArea::Panel, 0, area, scroll, content);
}

/// 结果列表中各行共用的绘制参数。
struct ResultContext {
    /// 结果列表区域，各行均裁剪在该区域内。
    area: Rect,
    /// 选中的结果行。
    selected: Option<ResultRow>,
    /// 焦点是否在结果列表上。
    focused: bool,
    /// 替换行是否展开；展开时悬停的行多显示一个「替换」按钮。
    replace_open: bool,
    /// 匹配行是否显示替换后的文本。
    replacement: bool,
    /// 单个数字的宽度（逻辑像素）。
    digit: f32,
    /// 行号列的宽度（逻辑像素），见 [`RowEnd::numbers`]。
    numbers: f32,
}

/// 绘制一个结果行：底色、内容，以及悬停或选中时右端浮出的按钮。
///
/// `index` 为该行在当前结果行中的序号，用于判断悬停；
/// 绘制折叠动画中的行时传 `None`，不显示悬停。
fn draw_result_row(
    app: &App,
    frame: &mut Frame<'_>,
    cx: &ResultContext,
    index: Option<usize>,
    kind: ResultRow,
    row: Rect,
) {
    let p = app.palette;
    let area = cx.area;
    let is_selected = cx.selected == Some(kind);
    let hovered = index.is_some_and(
        |index| matches!(app.pointer.hover, Hover::Result(i) | Hover::RowAction(i, _) if i == index),
    );
    let pill = Rect::new(row.x + 8.0, row.y + 1.0, row.w - 16.0, row.h - 2.0);

    let background = row_color(p, is_selected, cx.focused, hovered);
    if let Some(color) = background {
        frame.rect_clipped(pill, color, RADIUS_ROW, area);
    }

    // 写全名的那一行：按钮跟在全名之后，随名字展开滑出；名字在按钮之前截断，不用遮盖。
    let overflow = index.and_then(|i| app.panels.global.overflow.filter(|o| o.index == i));
    let (replace_rect, dismiss_rect) = row_actions(row, cx.replace_open);
    let end = RowEnd {
        slot: dismiss_rect,
        pad: (dismiss_rect.w - cx.digit) / 2.0,
        numbers: cx.numbers,
        covered: background.is_some(),
    };

    let moved = match (overflow, kind) {
        (Some(o), ResultRow::File(f)) => Some(moved_back(o.actions, actions_lag(app, frame, f))),
        _ => None,
    };
    match kind {
        ResultRow::File(f) => draw_file_row(app, frame, row, area, f, (&end, moved)),
        ResultRow::Match(f, m) => {
            draw_match_row(app, frame, row, area, (f, m), &end, cx.replacement);
        }
    }

    // 悬停或选中时，按钮浮现在右端匹配数、行号所在的位置：先用该行底色遮盖下方文本，
    // 遮盖区左侧带一段渐隐，再绘制按钮。
    let (replace_rect, dismiss_rect) = moved.unwrap_or((replace_rect, dismiss_rect));
    if let Some(background) = background {
        let left = replace_rect.map_or(dismiss_rect.x, |r| r.x) - 4.0;
        if moved.is_none() {
            let solid = over(p.popup_bg, background);
            let cover = Rect::new(left, pill.y, pill.right() - left, pill.h);
            let r = RADIUS_ROW;
            frame.rect_corners(cover, solid, [0.0, r, r, 0.0], area);
            fade(
                frame,
                Rect::new(left - COVER_FADE, pill.y, COVER_FADE, pill.h),
                solid,
                false,
            );
        }

        let hover_on =
            |action| index.is_some_and(|i| app.pointer.hover == Hover::RowAction(i, action));

        if let Some(rect) = replace_rect {
            draw_button(frame, p, rect, area, hover_on(RowAction::Replace), false);

            icons::replace_one(frame, center(rect), p.text_muted, area);
        }

        draw_button(
            frame,
            p,
            dismiss_rect,
            area,
            hover_on(RowAction::Dismiss),
            false,
        );

        icons::close(frame, center(dismiss_rect), 6.5, p.text_muted, area);
    }
}

/// 结果行右端那一格：匹配数、行号和悬停时浮出的按钮共用一条右边和一条上下中线，按钮出来时
/// 原地顶替匹配数、行号，左边的文字不动。
struct RowEnd {
    /// 最右一个按钮（「从结果中移除」）的位置（`row_actions`）。
    slot: Rect,
    /// 数字在这一格里左右留的空：一位数正好居中，多一位往左长一位。
    pad: f32,
    /// 行号一栏的宽度：结果里最大的行号有几位就留几位，各匹配行一样宽，右对齐。
    numbers: f32,
    /// 按钮浮出来了（悬停或选中），匹配数、行号不绘制。
    covered: bool,
}

/// 绘制第 `f` 个文件的结果行：箭头、文件图标、文件名、淡色的所在目录，右端是匹配数
/// （悬停时换成按钮）。
fn draw_file_row(
    app: &App,
    frame: &mut Frame<'_>,
    row: Rect,
    clip: Rect,
    f: usize,
    (end, moved): (&RowEnd, Option<(Option<Rect>, Rect)>),
) {
    let p = app.palette;
    let file = &app.panels.global.engine.results[f];
    let collapsed = app.panels.global.collapsed.contains(&file.path);
    let cy = icon_center_y(app, row, app.fonts.ui);
    // 折叠、展开途中，箭头按匹配行露出的比例旋转。
    let turned = match &app.panels.global.motion {
        Some(m) if m.rows.get(m.parent) == Some(&ResultRow::File(f)) => m.turned(),
        _ => f32::from(u8::from(!collapsed)),
    };
    let angle = turned * FRAC_PI_2;
    let chevron = (row.x + PANEL_CHEVRON, cy);
    icons::chevron_turned(frame, chevron, 6.0, angle, p.text_dim, clip);

    let Some(text) = file_text(app, f) else {
        return;
    };
    draw_file_icon(frame, p, (row.x + PANEL_LEFT, cy), text.name, clip);
    let x = row.x + PANEL_TEXT;

    // 右端的匹配数绘制为与按钮同高的胶囊，右边缘与按钮对齐；
    // 文字的右边界按胶囊计算，悬停时保持不变。
    let count = file.matches.len().to_string();
    let size = app.fonts.ui_small;
    let count_w = frame.measure(FontKind::Ui, size, &count);
    let slot = end.slot;
    let badge_w = (count_w + 2.0 * end.pad).max(slot.w);
    let badge = Rect::new(slot.right() - badge_w, slot.y, badge_w, slot.h);
    if !end.covered {
        frame.rect_clipped(badge, p.hover, slot.h / 2.0, clip);
        let x = badge.right() - end.pad - count_w;
        caps_label(frame, app, size, (x, badge), &count, p.text_muted, clip);
    }

    // 写全名：完整绘制，按展开到的宽度露出，在跟过来的按钮之前截断。
    if let Some((replace, dismiss)) = moved {
        let full = text_width(app, frame, &text);
        let shown = sync_name(app, f, &text, true, full).map_or(full, |v| v.width);
        let first = replace.unwrap_or(dismiss);
        let right = (x + shown).min(first.x - 4.0);
        let reveal = Rect::new(clip.x, clip.y, (right - clip.x).max(0.0), clip.h).intersect(clip);
        draw_text(app, frame, (x, row), &text, reveal);
        return;
    }

    let right = badge.x - end.pad;
    let name_max = (right - x).max(0.0);
    let name = ellipsize(frame, text.name, app.fonts.ui, name_max);
    let mut end_x = ui_text(frame, app, x, row, &name, p.text, clip);

    // 所在目录跟在文件名之后，放不下时从中间截短，仍放不下则不绘制。非 UTF-8 的文件不能替换，
    // 此处改为显示说明。
    let note_max = right - end_x - 8.0;
    if !text.note.is_empty() && note_max > 24.0 {
        let note = ellipsize(frame, text.note, app.fonts.ui_small, note_max);
        end_x = ui_text_small(frame, app, end_x + 8.0, row, &note, text.color, clip);
    }
    sync_name(app, f, &text, false, end_x - x);
}

/// 绘制第 `f` 个文件的第 `m` 处匹配的结果行：一小段原文，其中匹配的文本带底色；
/// 展开替换（`replacing`）时，匹配的文本被划掉，其后紧跟替换后的文本。
///
/// 右端是行号（悬停时换成按钮）。展开替换时调用方先用 `GlobalSearch::preview` 更新替换预览，
/// 这里只读缓存好的那份。
fn draw_match_row(
    app: &App,
    frame: &mut Frame<'_>,
    row: Rect,
    clip: Rect,
    (f, m): (usize, usize),
    end: &RowEnd,
    replacing: bool,
) {
    let p = app.palette;
    let found = &app.panels.global.engine.results[f].matches[m];
    // 右端的行号（不用打开就知道在哪一行）在定宽的一栏里右对齐，末位和文件行匹配数的末位
    // 对齐；文本让出整栏，行号几位都不影响文本。
    let numbers_right = end.slot.right() - end.pad;
    if !end.covered {
        let number = (found.line + 1).to_string();
        let size = app.fonts.ui_small;
        let x = numbers_right - frame.measure(FontKind::Ui, size, &number);
        caps_label(frame, app, size, (x, end.slot), &number, p.text_muted, clip);
    }
    let right = numbers_right - end.numbers - end.pad;
    let (preview, highlight) = (found.preview.as_str(), found.highlight.clone());
    let replaced = if replacing {
        app.panels
            .global
            .cached_preview()
            .map(|r| r.apply(preview, highlight.clone()))
    } else {
        None
    };

    let x0 = row.x + PANEL_TEXT;
    let (before, matched, after) = (
        &preview[..highlight.start],
        &preview[highlight.clone()],
        &preview[highlight.end..],
    );

    // 匹配位置离行首太远、整段放不下时，匹配之前的文本从左侧截去并换成「…」，使匹配的文本完整露出。
    let room = right - x0;
    let before_w = frame.measure(FontKind::Ui, app.fonts.ui, before);
    let matched_w = frame.measure(FontKind::Ui, app.fonts.ui, matched);
    // 展开替换时，替换后的文本也要露出来，因此计入宽度。
    let replaced_w = replaced
        .as_deref()
        .map_or(0.0, |r| frame.measure(FontKind::Ui, app.fonts.ui, r) + 3.0);
    let overflow = before_w + matched_w + replaced_w + 8.0 - room;
    let mut x = x0;
    let mut before = before;

    // 从前面按字符逐个去掉，直到放得下，使「…」之后不会出现半个字符。
    if overflow > 0.0 && !before.is_empty() {
        x = ui_text(frame, app, x0, row, "…", p.text_dim, clip) + 1.0;
        let need = overflow + (x - x0);
        let mut dropped = 0.0;

        for (i, c) in before.char_indices() {
            if dropped >= need {
                before = &before[i..];
                break;
            }

            dropped += frame.measure(FontKind::Ui, app.fonts.ui, c.encode_utf8(&mut [0; 4]));

            if i + c.len_utf8() == before.len() {
                before = "";
            }
        }
    }

    let clip = Rect::new(clip.x, clip.y, (right - clip.x).max(0.0), clip.h);
    x = ui_text(frame, app, x, row, before, p.text_muted, clip);

    let mark = Rect::new(x - 1.0, row.y + 5.0, matched_w + 2.0, row.h - 10.0);
    let mark_color = if replaced.is_some() {
        p.error.alpha(0.18)
    } else {
        p.search_match
    };
    frame.rect_clipped(mark, mark_color, 3.0, clip);
    let end = ui_text(frame, app, x, row, matched, p.text, clip);

    if let Some(new) = &replaced {
        // 被替换的文本划一道删除线，其后是替换后的文本，底色为绿色调。
        let strike = Rect::new(x, row.y + row.h / 2.0, end - x, 1.0);
        frame.rect_clipped(strike, p.text_muted, 0.0, clip);
        let new_w = frame.measure(FontKind::Ui, app.fonts.ui, new);
        let added = Rect::new(end + 1.0, row.y + 5.0, new_w + 2.0, row.h - 10.0);
        frame.rect_clipped(added, p.syntax_string.alpha(0.20), 3.0, clip);
        x = ui_text(frame, app, end + 2.0, row, new, p.text, clip) + 1.0;
    } else {
        x = end;
    }

    ui_text(frame, app, x, row, after, p.text_muted, clip);
}

