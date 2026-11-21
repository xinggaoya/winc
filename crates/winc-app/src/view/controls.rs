//! 设置和开始页共用的控件的绘制：点击后弹出菜单的值、开关、分段控件的底槽和凸起的一段。
//!
//! 这里只绘制，控件的位置、状态和点击区域由调用方决定。

use super::*;

/// 绘制点击后弹出菜单的值：平时是一段右对齐的淡色文本（未选择时 `dim` 为真，颜色更淡），
/// 其后一对上下小箭头表示可选；悬停时铺悬停底色。
///
/// 不绘制边框，样式与字体名等其他设置项一致。
pub(in crate::view) fn popup_value(
    app: &App,
    frame: &mut Frame<'_>,
    rect: Rect,
    text: &str,
    dim: bool,
    hovered: bool,
    clip: Rect,
) {
    let p = app.palette;
    if hovered {
        frame.rect_clipped(rect, p.hover, RADIUS_ROW, clip);
    }
    let area = Rect::new(
        rect.x + 10.0,
        rect.y,
        (rect.w - 10.0 - SETTINGS_GLYPH_PAD).max(0.0),
        rect.h,
    );
    let arrows = (area.right() - 3.0, rect.y + rect.h / 2.0);
    icons::up_down(frame, arrows, p.text_dim, clip);

    let room = Rect::new(area.x, area.y, (area.w - 16.0).max(0.0), area.h);
    let text = ellipsize(frame, text, app.fonts.ui, room.w);
    let w = frame.measure(FontKind::Ui, app.fonts.ui, &text);
    let color = if dim { p.text_dim } else { p.text_muted };
    ui_text(frame, app, room.right() - w, room, &text, color, room);
}

/// 绘制开关 `track`：圆角底色，开启时为强调色、圆钮在右侧，关闭时为灰色、圆钮在左侧。
///
/// `t` 是开启程度：0 为关闭，1 为开启，切换途中在两者之间，圆钮和强调色随之过渡。
pub(in crate::view) fn switch(frame: &mut Frame<'_>, p: &Palette, track: Rect, t: f32, clip: Rect) {
    let radius = track.h / 2.0;
    frame.rect_clipped(track, p.switch_off, radius, clip);
    if t > 0.0 {
        frame.rect_clipped(track, p.accent.alpha(p.accent.a * t), radius, clip);
    }
    let knob = track.h - 4.0;
    let left = track.x + 2.0;
    let x = left + (track.right() - 2.0 - knob - left) * t;
    let knob = Rect::new(x, track.y + 2.0, knob, knob);
    frame.rect_clipped(knob, p.switch_knob, knob.h / 2.0, clip);
}

/// 绘制分段控件的底槽 `track` 和其中凸起的一段 `slot`（没有选中项时为 `None`），各段的文字由
/// 调用方压在上面。`under` 是控件下面的底色，凸起段的颜色据此算出。
pub(in crate::view) fn segment_track(
    frame: &mut Frame<'_>,
    p: &Palette,
    track: Rect,
    under: Color,
    slot: Option<Rect>,
    clip: Rect,
) {
    frame.rect_clipped(track, p.hover, RADIUS_ROW + 2.0, clip);
    let track_bg = super::controls::over(under, p.hover);
    let (raised, edge) = super::controls::segment_colors(p, track_bg);
    if let Some(slot) = slot {
        frame.rect_clipped(slot, edge, RADIUS_ROW, clip);
        frame.rect_clipped(slot.inset(0.5, 0.5), raised, RADIUS_ROW - 0.5, clip);
    }
}
/// 返回 `fg` 叠在不透明的 `bg` 上得到的颜色，结果不透明。
///
/// 用于绘制与下层完全相同的底色，以遮盖其上的文本。
pub(in crate::view) fn over(bg: Color, fg: Color) -> Color {
    let mix = |b: f32, f: f32| b + (f - b) * fg.a;
    Color {
        r: mix(bg.r, fg.r),
        g: mix(bg.g, fg.g),
        b: mix(bg.b, fg.b),
        a: 1.0,
    }
}

/// 分段控件里凸起那一段的底色和细边：浅色配色是白底、淡黑边；深色配色是比底槽 `track` 亮一档的
/// 灰、淡白边。
///
/// 都是不透明色，叠在底槽上不会透出下面。
pub(in crate::view) fn segment_colors(p: &Palette, track: Color) -> (Color, Color) {
    let white = Color::hex(0xFFFFFF);
    if p.dark {
        let raised = over(track, white.alpha(0.09));
        (raised, over(track, white.alpha(0.14)))
    } else {
        (white, over(track, Color::hex(0x000000).alpha(0.08)))
    }
}

