//! 左右两栏里的面板：哪些面板现在能用、各栏里有哪些面板、面板的位置和焦点。
//!
//! 窗口分成三栏：左栏、右栏和中间那栏。中间一栏固定是编辑区和终端；左右两栏放各个面板，
//! 哪面板在哪一栏、各栏里的顺序由用户配置决定（见 [`PanelLayout`]）。

use super::*;
use crate::layout::PanelSlot;

/// 去掉两栏里不再显示的面板：全局搜索改用浮在编辑区上的弹窗，不再占侧栏的位置。
///
/// 配置里旧的 `"search"` 读进来时丢掉：它在两栏里没有位置，放进去只会让设置页列出
/// 一个点不动的面板。
pub(crate) fn without_search(panels: &[Panel]) -> Vec<Panel> {
    panels
        .iter()
        .copied()
        .filter(|panel| *panel != Panel::Search)
        .collect()
}

/// 右栏展开与收起的动画。
#[derive(Debug, Clone, Copy)]
pub(in crate::app) struct RightMotion {
    /// 开始时露出的比例：0 为隐藏，1 为完全展开。
    pub(in crate::app) from: f32,
    /// 动画的时钟。
    pub(in crate::app) clock: crate::app::animation::Clock,
}

impl App {
    /// 面板 `panel` 现在能不能显示。
    ///
    /// Git 的提交视图在 Git 模块开着时可用；模块关着时，目录是仓库的话仍然列出来，写明模块
    /// 关着、用户到设置里打开。文件树一直都有。
    ///
    /// 全局搜索不算：它改用浮在编辑区上的弹窗，不占两栏的位置。
    pub(crate) fn panel_available(&self, panel: Panel) -> bool {
        match panel {
            Panel::Files => true,
            Panel::Commit => self.git_enabled() || self.in_git_repository(),
            Panel::Search => false,
        }
    }

    /// 当前打开的目录是不是 Git 仓库：看目录里有没有 `.git`（目录或文件都行）。
    fn in_git_repository(&self) -> bool {
        self.editor
            .root()
            .is_some_and(|root| winc_backend::fs::exists(root.join(".git")))
    }

    /// 当前左右两栏里各有哪些面板：按配置的顺序，跳过现在不能显示的面板。
    pub(crate) fn columns(&self) -> (Vec<Panel>, Vec<Panel>) {
        let left = self
            .panels
            .layout
            .left
            .iter()
            .copied()
            .filter(|panel| self.panel_available(*panel))
            .collect();
        let right = self
            .panels
            .layout
            .right
            .iter()
            .copied()
            .filter(|panel| self.panel_available(*panel))
            .collect();
        (left, right)
    }

    /// 右栏露出的比例：0 为隐藏，1 为完全展开，动画中介于两者之间。
    ///
    /// 右栏里没有能显示的面板时为 0（不占地方）。
    pub(crate) fn right_shown(&self) -> f32 {
        let has_panels = self
            .panels
            .layout
            .right
            .iter()
            .any(|panel| self.panel_available(*panel));
        let target = if self.panels.right_hidden || !has_panels {
            0.0
        } else {
            1.0
        };
        let Some(motion) = self.panels.right_motion else {
            return target;
        };
        motion.clock.between(motion.from, target)
    }

    /// 面板 `panel` 在某一栏里的位置；现在没有显示时返回 `None`。
    pub(crate) fn panel_slot(&self, panel: Panel) -> Option<PanelSlot> {
        let l = &self.layout;
        l.left_panels[..l.left_count]
            .iter()
            .chain(l.right_panels[..l.right_count].iter())
            .find(|slot| slot.panel == Some(panel))
            .copied()
    }

    /// 面板 `panel` 所在栏的卡片位置；面板不在任何一栏里时返回空矩形。
    pub(crate) fn panel_card(&self, panel: Panel) -> Rect {
        self.panel_slot(panel)
            .map(|slot| slot.card)
            .unwrap_or_default()
    }

    /// 提交面板的卡片位置；不在任何一栏里时返回空矩形。
    pub(crate) fn commit_card(&self) -> Rect {
        self.panel_card(Panel::Commit)
    }

    /// 收起或展开右栏（带动画）。
    ///
    /// 动画中途反向时从当前位置继续。
    pub(in crate::app) fn toggle_right_column(&mut self) {
        let from = self.right_shown();
        self.panels.right_hidden = !self.panels.right_hidden;
        self.panels.right_motion = self
            .clock(crate::app::animation::SIDEBAR_ANIMATION)
            .map(|clock| RightMotion { from, clock });
        self.relayout();
        self.request_redraw();
    }

    /// 确保面板 `panel` 在某一栏里显示：现在没有显示时把它放到左栏末尾。
    ///
    /// 用户可以把面板移到右栏或者藏起来；用命令打开某个面板时（提交），面板没有地方
    /// 放就重新放到左栏末尾，免得命令没有反应。
    ///
    /// 全局搜索不在两栏里：它改用浮在编辑区上的弹窗，不占侧栏的位置。
    pub(in crate::app) fn show_panel(&mut self, panel: Panel) {
        if panel == Panel::Search {
            return;
        }

        let layout = &self.panels.layout;
        if layout.left.contains(&panel) || layout.right.contains(&panel) {
            return;
        }
        self.panels.layout.left.push(panel);
        self.panels_changed();
    }

    /// 把焦点交给面板 `panel`：展开它所在的栏，焦点落到面板里。
    ///
    /// 面板现在没有显示时（例如 Git 的提交视图，但目录不在仓库里）只记下焦点，面板不显示。
    /// 全局搜索不在两栏里：它改用浮在编辑区上的弹窗，焦点给它就是打开弹窗。
    pub(in crate::app) fn focus_panel(&mut self, panel: Panel) {
        if panel == Panel::Search {
            self.open_global_search(false);
            return;
        }

        self.set_sidebar_view(panel);
        match panel {
            Panel::Files => self.focus = Focus::Tree,
            Panel::Commit => self.focus = self.commit_view_focus(),
            // 全局搜索已在上面返回。
            Panel::Search => {}
        }
    }
}
