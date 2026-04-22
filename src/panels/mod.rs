pub mod branch_panel;
pub mod filelist_panel;
pub mod log_panel;
pub mod stash_panel;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::any::Any;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    BackToMain,
    ShowBranchPanel,
    ShowLogPanel,
    ShowStashPanel,
    Stage,
    StageAll,
    Unstage,
    UnstageAll,
    Discard,
    CommitDialog,
    Commit(String),
    AmendCommit,
    CheckoutBranch(String),
    PushCurrent,
    FetchAll,
    Help,
    // Stash
    Stash,
    StashPop(usize),
    StashApply(usize),
    StashDrop(usize),
    // Shelve
    ShelveApply(String),
    ShelveDrop(String),
    ShelveCreate,
    // Log
    CherryPick(String),
    CopyHash(String),
    SearchLog(String),
    // Branch
    CreateBranch(String),
    CreateBranchDialog,
    DeleteBranch(String),
    MergeBranch(String),
    RebaseBranch(String),
    PullCurrent,
    ShowDiff(String),
    // Tags
    ShowTags,
    CreateTag(String),
    DeleteTag(String),
}

pub trait Panel {
    fn focus(&mut self);
    fn blur(&mut self);
    fn render(&mut self, f: &mut Frame, area: ratatui::layout::Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;
    fn title(&self) -> &str;
    fn refresh(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
