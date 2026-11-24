//! 三栏布局中左右两栏的几何：栏的位置、栏里各个面板的位置。
//!
//! 窗口分成三栏：左栏放文件树等面板，中间是编辑区和终端（固定，不参与配置），右栏放 Git
//! 等面板。哪面板放在哪一栏、各栏里的顺序由用户配置决定（见 [`crate::app::PanelLayout`]）。

use winc_config::Panel;
use winc_render::Rect;

/// 一栏里最多能放的面板数：面板的种类数。
pub const MAX_PANELS: usize = winc_config::Panel::ALL.len();

/// 一栏中的一个面板。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PanelSlot {
    /// 面板的种类。
    pub panel: Option<Panel>,
    /// 面板卡片的位置（含标题行）。
    pub card: Rect,
    /// 标题行的位置。
    pub header: Rect,
    /// 正文的位置（标题行之下）。
    pub body: Rect,
}

/// 面板标题行的高度（逻辑像素）。
pub const PANEL_HEADER: f32 = 26.0;

/// 栏里各个面板之间留的间距（逻辑像素）。
pub const PANEL_GAP: f32 = 4.0;

/// 返回栏 `card` 里各个面板的位置，顺序同 `panels`。
///
/// `top` 是栏里第一个面板的上沿（左栏在项目名一行之下，右栏从顶栏之下开始）。栏里的面板
/// 从上到下排列，各占一样高。空着的格子 `panel` 为 `None`。
pub fn panel_slots(card: Rect, top: f32, panels: &[Panel]) -> [PanelSlot; MAX_PANELS] {
    let mut slots = [PanelSlot::default(); MAX_PANELS];
    let count = panels.len().min(MAX_PANELS);
    if count == 0 {
        return slots;
    }

    let available = (card.bottom() - top).max(0.0);
    let each = (available / count as f32).max(0.0);

    let mut y = top;
    for (index, &panel) in panels.iter().take(count).enumerate() {
        let header = Rect::new(card.x, y, card.w, PANEL_HEADER.min(each));
        let body = Rect::new(card.x, header.bottom(), card.w, (each - header.h).max(0.0));
        slots[index] = PanelSlot {
            panel: Some(panel),
            card: Rect::new(card.x, y, card.w, each),
            header,
            body,
        };
        y += each + PANEL_GAP;
    }
    slots
}
