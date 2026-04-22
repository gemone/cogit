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
    RenameBranch(String, String),
    RenameBranchDialog(String), // old_name to pass to the dialog
    DeleteBranch(String),
    MergeBranch(String),
    RebaseBranch(String),
    PullCurrent,
    ShowDiff(String),
    // Diff panel for arbitrary refs
    ShowRefDiff(String), // "from..to" format
    // Worktree
    ShowWorktrees,
    CreateWorktree(String, String), // path, branch
    RemoveWorktree(String),
    // Rebase pull
    PullRebase,
    // Tags
    ShowTags,
    CreateTag(String),
    DeleteTag(String),
    // Reset
    Reset(String, String), // path, mode ("soft"|"hard"|"mixed")
    ResetDialog(String),   // mode for dialog
    // WIP
    WipCommit,
    // Gitignore
    ShowGitignore,
    GitignoreAdd(String),
    GitignoreRemove(String),
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
