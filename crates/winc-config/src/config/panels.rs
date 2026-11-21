//! 侧栏面板的种类，以及各栏的默认排法。
//!
//! 界面分成三栏：左栏放文件树等面板，中间是编辑区和终端，右栏放 Git 等面板。哪面板
//! 放在哪一栏、各栏里的顺序，由用户配置决定（见 [`Config::panels_left`] 和
//! [`Config::panels_right`]）；中间一栏固定，不参与配置。

/// 侧栏面板的种类。
///
/// 面板放在左栏还是右栏、各栏里的顺序，由配置决定；中间一栏固定是编辑区和终端。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Panel {
    /// 文件树（底部带当前文件的符号大纲）。
    #[default]
    Files,
    /// 全局搜索。
    Search,
    /// Git 的提交视图：改动列表和提交信息。
    Commit,
}

impl Panel {
    /// 全部面板种类，顺序为配置里列出的默认顺序。
    pub const ALL: [Panel; 3] = [Panel::Files, Panel::Search, Panel::Commit];

    /// 左栏的默认排法：文件树。
    pub const DEFAULT_LEFT: [Panel; 1] = [Panel::Files];

    /// 右栏的默认排法：Git 的提交视图。
    pub const DEFAULT_RIGHT: [Panel; 1] = [Panel::Commit];

    /// 面板在配置文件里的写法（`files`、`search`、`commit`）。
    pub fn key(self) -> &'static str {
        match self {
            Panel::Files => "files",
            Panel::Search => "search",
            Panel::Commit => "commit",
        }
    }

    /// 按配置文件里的写法读出面板种类；不认识时返回 `None`。
    pub fn from_key(key: &str) -> Option<Panel> {
        match key {
            "files" => Some(Panel::Files),
            "search" => Some(Panel::Search),
            "commit" => Some(Panel::Commit),
            _ => None,
        }
    }
}
