//! ActionName: string identifiers for `panels::Action` variants that can be
//! bound to keys. Parameterized actions (e.g. `Commit(String)`) are
//! intentionally not reachable from the keymap; they stay hardcoded in
//! panels for now.

use crate::panels::Action;

/// A string name for a keymap-bindable action, e.g. `"quit"`, `"stage"`,
/// `"commit_dialog"`, `"show_branches"`.
pub type ActionName = &'static str;

/// All action names and their `Action` mapping, in registration order.
/// Used by the default keymap and by tests.
pub const ACTION_REGISTRY: &[ActionName] = &[
    "quit",
    "help",
    "back_to_main",
    "show_branches",
    "show_log",
    "show_stash",
    "show_remote",
    "show_shelve",
    "stage",
    "stage_all",
    "unstage",
    "unstage_all",
    "stage_toggle",
    "diff",
    "commit_dialog",
    "command_line",
    "stash_create",
    "stash_pop",
    "stash_apply",
    "stash_drop",
    "shelve_apply",
    "shelve_drop",
    "branch_create_dialog",
    "branch_checkout",
    "branch_delete",
    "branch_rename_dialog",
    "branch_merge",
    "branch_rebase",
    "fetch_all",
    "push_current",
    "pull_current",
    "log_copy_hash",
    "log_cherry_pick",
    "log_search",
];

/// Convert an `ActionName` to its `Action`. Parameterized actions that need
/// panel state (selected file, selected branch, ...) map to `None` here —
/// those stay panel-hardcoded; their names exist so the default table can
/// document panel keys, and keymap hits for them fall through to panels.
pub fn action_from_name(name: &str) -> Option<Action> {
    Some(match name {
        "quit" => Action::Quit,
        "help" => Action::Help,
        "back_to_main" => Action::BackToMain,
        "show_branches" => Action::ShowBranchPanel,
        "show_log" => Action::ShowLogPanel,
        "show_stash" => Action::ShowStashPanel,
        "show_remote" => Action::ShowRemotePanel,
        "show_shelve" => Action::ShowShelvePanel,
        "stage" => Action::Stage,
        "stage_all" => Action::StageAll,
        "unstage" => Action::Unstage,
        "unstage_all" => Action::UnstageAll,
        "commit_dialog" => Action::CommitDialog,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_are_unique() {
        let mut names: Vec<&str> = ACTION_REGISTRY.to_vec();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "duplicate action names in registry");
    }

    #[test]
    fn core_actions_map() {
        assert!(matches!(action_from_name("quit"), Some(Action::Quit)));
        assert!(matches!(action_from_name("stage"), Some(Action::Stage)));
        assert!(matches!(
            action_from_name("stage_all"),
            Some(Action::StageAll)
        ));
        assert!(matches!(
            action_from_name("commit_dialog"),
            Some(Action::CommitDialog)
        ));
        assert!(matches!(
            action_from_name("show_branches"),
            Some(Action::ShowBranchPanel)
        ));
        assert!(matches!(action_from_name("help"), Some(Action::Help)));
    }

    #[test]
    fn unknown_and_parameterized_actions_do_not_map() {
        assert!(action_from_name("nope").is_none());
        // Parameterized actions are not keymap-dispatchable.
        assert!(action_from_name("commit").is_none());
        assert!(action_from_name("reset").is_none());
    }
}
