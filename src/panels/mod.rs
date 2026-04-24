pub mod branch_panel;
pub mod filelist_panel;
pub mod log_panel;
pub mod remote_panel;
pub mod shelve_panel;
pub mod stash_panel;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::any::Any;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    BackToMain,
    OpenCommandPalette,
    NextView,
    PrevView,
    ShowBranchPanel,
    ShowLogPanel,
    ShowStashPanel,
    ShowRemotePanel,
    Stage,
    StageAll,
    Unstage,
    UnstageAll,
    ToggleStage,
    Discard,
    CommitDialog,
    Commit(String),
    AmendCommit,
    CheckoutBranch(String),
    CheckoutRemoteBranch(String),
    PushCurrent,
    FetchAll,
    Help,
    // Stash
    Stash,
    StashPop(usize),
    StashApply(usize),
    StashDrop(usize),
    // Remote
    AddRemote(String, String),
    RemoveRemote(String),
    RenameRemote(String, String),
    FetchRemote(String),
    ShowRemoteBranches(String),
    // Shelve
    ShowShelvePanel,
    ShelveCreate(String, bool),
    ShelveApply(usize, bool),
    ShelveDrop(usize),
    // Old shelve actions (keep for compatibility)
    ShelveApplyOld(String),
    ShelveDropOld(String),
    ShelveCreateOld,
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
    RebaseContinue,
    RebaseAbort,
    RebaseSkip,
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
    SetKeymapPreset(String),
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
