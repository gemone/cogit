use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
};

use crate::gitops::{GitError, Repo};

pub mod cmdbar;
pub mod diffviewer;
pub mod filelist;
pub mod sidebar;

pub use cmdbar::CmdbarPanel;
pub use diffviewer::DiffViewerPanel;
pub use filelist::FileListPanel;
pub use sidebar::SidebarPanel;

use crate::app::styles::Styles;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    FocusSidebar,
    FocusFilelist,
    FocusDiff,
    Stage,
    Unstage,
    StageAll,
    UnstageAll,
    Discard(String),
    ToggleIgnore,
    ToggleUntracked,
    Checkout,
    NewBranch,
    DeleteBranch,
    Refresh,
    Quit,
    CommandPalette,
    Help,
    EnterMode(Mode),
    OpenDiff(String),
    IgnoreFile(String),
    Search,
    UpdateDiff(String),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Visual,
    Command,
    Insert,
}

pub trait Panel {
    fn focus(&mut self);
    fn blur(&mut self);
    fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;
    fn title(&self) -> &str;
    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError>;
}
