use std::any::Any;

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::gitops::{GitError, Repo};

pub mod branch_panel;
pub mod cmdbar;
pub mod diffviewer;
pub mod filelist;
pub mod log_panel;
pub mod sidebar;

pub use branch_panel::BranchPanel;
pub use cmdbar::CmdbarPanel;
pub use diffviewer::DiffViewerPanel;
pub use filelist::FileListPanel;
pub use log_panel::LogPanel;
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
    CheckoutBranch(String),
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
    CommitDialog,
    Commit(String),
    AmendCommit,

    // Branch panel actions
    CheckoutSmart(String),
    ForceCheckout(String),
    NewBranchDialog,
    CreateBranch(String),
    DeleteBranchConfirm(String),
    FetchRemote(String),
    FetchAll,
    PullDialog,
    PullMerge,
    PullRebase,
    PushCurrent,
    MergeBranch(String),
    RebaseOnto(String),
    ShowBranchPanel,
    BackToMain,
    RenameBranch {
        old_name: String,
        new_name: String,
    },
    Stash,
    StashPop,

    // Log panel actions
    ShowLogPanel,
    CherryPick(String),
    CopyHash(String),
    SearchCommits,

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
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
