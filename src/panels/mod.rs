pub mod branch_panel;
pub mod console_panel;
pub mod filelist_panel;
pub mod log_panel;
pub mod rebase_panel;
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
    EnterNormalMode,
    EnterEditMode,
    EnterVisualMode,
    HideActivePane,
    ShowAllPanes,
    NextView,
    PrevView,
    GrowPaneWidth,
    ShrinkPaneWidth,
    GrowPaneHeight,
    ShrinkPaneHeight,
    ResetLayout,
    SaveLayoutLocal,
    SaveLayoutGlobal,
    ShowFilesPanel,
    ShowBranchPanel,
    ShowLogPanel,
    ShowStashPanel,
    ShowRemotePanel,
    ShowRebasePanel,
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
    SetKeymapPreset(crate::config::KeymapPreset),
    Undo,
    Revert(String),
    ExecuteRebase(String, Vec<crate::gitops::types::RebaseTodo>),
    StartRebase(String),
    ShowConsolePanel,
}

pub trait Panel {
    fn focus(&mut self);
    fn blur(&mut self);
    fn render(&mut self, f: &mut Frame, area: ratatui::layout::Rect, shortcut: Option<&str>);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;
    fn title(&self) -> &str;
    fn refresh(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub fn format_panel_title(title: &str, shortcut: Option<&str>) -> String {
    let title = title.trim();
    match shortcut.map(str::trim).filter(|value| !value.is_empty()) {
        Some(shortcut) => format!(" [{}] {} ", shortcut, title),
        None => format!(" {} ", title),
    }
}

pub fn format_section_title(title: &str) -> String {
    format_panel_title(title, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_panel_title_prefixes_shortcut_hint() {
        assert_eq!(format_panel_title("Branches", Some("1")), " [1] Branches ");
        assert_eq!(format_panel_title("Remotes", Some("R")), " [R] Remotes ");
    }

    #[test]
    fn format_panel_title_keeps_plain_title_without_shortcut_hint() {
        assert_eq!(format_panel_title("Files", None), " Files ");
    }

    #[test]
    fn format_section_title_uses_shared_plain_panel_style() {
        assert_eq!(format_section_title("Detail"), " Detail ");
        assert_eq!(
            format_section_title("Shelve Diff (Esc/q:close j/k:scroll)"),
            " Shelve Diff (Esc/q:close j/k:scroll) "
        );
    }
}
