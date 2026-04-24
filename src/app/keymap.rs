use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use crate::{
    config::{CogitConfig, KeymapOverrides, KeymapPreset},
    panels::Action,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyContext {
    Global,
    Main,
    Branches,
    Log,
    Stash,
    Remote,
    Shelve,
    Navigation,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMotion {
    Up,
    Down,
    Top,
    Bottom,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone)]
pub struct KeyBindingHint {
    pub key: String,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
struct BindingSpec {
    id: &'static str,
    key: String,
    description: &'static str,
    action: Option<Action>,
}

#[derive(Debug, Clone)]
struct KeymapState {
    preset: KeymapPreset,
    overrides: KeymapOverrides,
}

#[derive(Debug, Clone)]
pub struct KeymapManager {
    state: Arc<RwLock<KeymapState>>,
}

impl KeymapManager {
    pub fn new(config: &CogitConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(KeymapState {
                preset: config.keymap.preset,
                overrides: config.keymap.overrides.clone(),
            })),
        }
    }

    pub fn preset(&self) -> KeymapPreset {
        self.state.read().expect("keymap lock poisoned").preset
    }

    pub fn set_preset(&self, preset: KeymapPreset) {
        self.state.write().expect("keymap lock poisoned").preset = preset;
    }

    pub fn set_overrides(&self, overrides: KeymapOverrides) {
        self.state.write().expect("keymap lock poisoned").overrides = overrides;
    }

    pub fn preset_name(&self) -> &'static str {
        self.preset().as_str()
    }

    /// Resolve a key press to an action. Skips hint-only bindings (action: None).
    pub fn resolve(&self, context: KeyContext, key: KeyEvent) -> Option<Action> {
        let pressed = key_label(key);
        let state = self.state.read().expect("keymap lock poisoned");
        for spec in bindings_for(state.preset, context, &state.overrides)
            .into_iter()
            .chain(bindings_for(state.preset, KeyContext::Global, &state.overrides).into_iter())
        {
            if spec.key.eq_ignore_ascii_case(&pressed) {
                if let Some(action) = spec.action.clone() {
                    return Some(action);
                }
            }
        }
        None
    }

    pub fn resolve_motion(&self, key: KeyEvent) -> Option<ListMotion> {
        let pressed = key_label(key);
        let state = self.state.read().expect("keymap lock poisoned");
        for spec in bindings_for(state.preset, KeyContext::Navigation, &state.overrides) {
            if spec.key.eq_ignore_ascii_case(&pressed) {
                return match spec.id {
                    "nav_up" | "nav_up_alt" => Some(ListMotion::Up),
                    "nav_down" | "nav_down_alt" => Some(ListMotion::Down),
                    "nav_top" => Some(ListMotion::Top),
                    "nav_bottom" => Some(ListMotion::Bottom),
                    "nav_page_up" | "nav_page_up_alt" => Some(ListMotion::PageUp),
                    "nav_page_down" | "nav_page_down_alt" => Some(ListMotion::PageDown),
                    _ => None,
                };
            }
        }
        None
    }

    pub fn bindings_for(&self, context: KeyContext) -> Vec<KeyBindingHint> {
        let state = self.state.read().expect("keymap lock poisoned");
        bindings_for(state.preset, context, &state.overrides)
            .into_iter()
            .map(|spec| KeyBindingHint {
                key: spec.key,
                description: spec.description,
            })
            .collect()
    }

    pub fn override_binding(&self, context: KeyContext, action_id: &str, key: String) {
        let mut state = self.state.write().expect("keymap lock poisoned");
        let context_overrides = overrides_mut(&mut state.overrides, context);
        context_overrides.insert(action_id.to_string(), key);
    }
}

fn overrides_mut(overrides: &mut KeymapOverrides, context: KeyContext) -> &mut BTreeMap<String, String> {
    match context {
        KeyContext::Global => &mut overrides.global,
        KeyContext::Main => overrides.views.entry("main".to_string()).or_default(),
        KeyContext::Branches => overrides.views.entry("branches".to_string()).or_default(),
        KeyContext::Log => overrides.views.entry("log".to_string()).or_default(),
        KeyContext::Stash => overrides.views.entry("stash".to_string()).or_default(),
        KeyContext::Remote => overrides.views.entry("remote".to_string()).or_default(),
        KeyContext::Shelve => overrides.views.entry("shelve".to_string()).or_default(),
        KeyContext::Navigation => overrides.views.entry("navigation".to_string()).or_default(),
        KeyContext::Command => overrides.views.entry("command".to_string()).or_default(),
    }
}

fn bindings_for(preset: KeymapPreset, context: KeyContext, overrides: &KeymapOverrides) -> Vec<BindingSpec> {
    let mut defaults = match preset {
        KeymapPreset::Vim => vim_bindings(context),
        KeymapPreset::Helix => helix_bindings(context),
    };

    let override_map = match context {
        KeyContext::Global => Some(&overrides.global),
        KeyContext::Main => overrides.views.get("main"),
        KeyContext::Branches => overrides.views.get("branches"),
        KeyContext::Log => overrides.views.get("log"),
        KeyContext::Stash => overrides.views.get("stash"),
        KeyContext::Remote => overrides.views.get("remote"),
        KeyContext::Shelve => overrides.views.get("shelve"),
        KeyContext::Navigation => overrides.views.get("navigation"),
        KeyContext::Command => overrides.views.get("command"),
    };

    if let Some(map) = override_map {
        for spec in &mut defaults {
            if let Some(key) = map.get(spec.id) {
                spec.key = key.clone();
            }
        }
    }

    defaults
}

fn vim_bindings(context: KeyContext) -> Vec<BindingSpec> {
    match context {
        KeyContext::Global => vec![
            binding("open_command", ":", "Open command palette", Some(Action::OpenCommandPalette)),
            binding("help", "?", "Show which-key/help", Some(Action::Help)),
            binding("quit", "q", "Quit", Some(Action::Quit)),
            binding("view_branches", "1", "Open branches panel", Some(Action::ShowBranchPanel)),
            binding("view_log", "2", "Open log panel", Some(Action::ShowLogPanel)),
            binding("view_stash", "4", "Open stash/shelve panel", Some(Action::ShowStashPanel)),
            binding("view_remote", "R", "Open remotes panel", Some(Action::ShowRemotePanel)),
            binding("view_shelve", "W", "Open shelves panel", Some(Action::ShowShelvePanel)),
        ],
        KeyContext::Main => vec![
            binding("stage", "s", "Stage selected file", Some(Action::Stage)),
            binding("stage_all", "S", "Stage all files", Some(Action::StageAll)),
            binding("unstage", "u", "Unstage selected file", Some(Action::Unstage)),
            binding("unstage_all", "U", "Unstage all files", Some(Action::UnstageAll)),
            binding("toggle_stage", "Space", "Toggle stage/unstage", Some(Action::ToggleStage)),
            binding("discard", "d", "Discard selected file", Some(Action::Discard)),
            binding("commit", "c", "Open commit dialog", Some(Action::CommitDialog)),
            binding("open_diff", "Enter", "Open diff popup", None),
            binding("reset_dialog", "Ctrl+u", "Open reset dialog", Some(Action::ResetDialog("mixed".to_string()))),
        ],
        KeyContext::Branches => vec![
            binding("checkout", "Enter", "Checkout selected branch", None),
            binding("create_branch", "n", "Create branch", Some(Action::CreateBranchDialog)),
            binding("rename_branch", "R", "Rename branch", None),
            binding("delete_branch", "d", "Delete branch", None),
            binding("fetch", "f", "Fetch all remotes", Some(Action::FetchAll)),
            binding("push", "p", "Push current branch", Some(Action::PushCurrent)),
            binding("pull", "P", "Pull current branch", Some(Action::PullCurrent)),
            binding("merge", "m", "Merge branch", None),
            binding("rebase", "r", "Rebase branch", None),
            binding("remote_checkout", "o", "Checkout remote branch", None),
            binding("rebase_continue", "c", "Continue rebase", Some(Action::RebaseContinue)),
            binding("rebase_abort", "a", "Abort rebase", Some(Action::RebaseAbort)),
            binding("rebase_skip", "s", "Skip rebase step", Some(Action::RebaseSkip)),
            binding("search", "/", "Search branches", None),
            binding("back", "q", "Back to main view", Some(Action::BackToMain)),
        ],
        KeyContext::Log => vec![
            binding("copy_hash", "y", "Copy commit hash", None),
            binding("cherry_pick", "c", "Cherry-pick commit", None),
            binding("search", "/", "Search commits", None),
            binding("back", "q", "Back to main view", Some(Action::BackToMain)),
        ],
        KeyContext::Stash => vec![
            binding("toggle_tab", "Tab", "Switch stash/shelve tab", None),
            binding("pop", "Enter", "Pop selected stash entry", None),
            binding("apply", "a", "Apply selected stash entry", None),
            binding("drop", "d", "Drop selected stash entry", None),
            binding("stash", "s", "Create stash", Some(Action::Stash)),
            binding("back", "q", "Back to main view", Some(Action::BackToMain)),
        ],
        KeyContext::Remote => vec![
            binding("add", "a", "Add remote", None),
            binding("delete", "d", "Delete remote", None),
            binding("rename", "r", "Rename remote", None),
            binding("fetch", "u", "Fetch remote", None),
            binding("show_branches", "Enter", "Show remote branches", None),
            binding("back", "q", "Back to main view", Some(Action::BackToMain)),
        ],
        KeyContext::Shelve => vec![
            binding("new", "n", "Create shelve", None),
            binding("toggle_staged", "s", "Toggle include staged", None),
            binding("pop", "p", "Pop selected shelve", None),
            binding("apply", "a", "Apply selected shelve", None),
            binding("drop", "d", "Drop selected shelve", None),
            binding("diff", "Enter", "View shelve diff", None),
            binding("back", "q", "Back to main view", Some(Action::BackToMain)),
        ],
        KeyContext::Navigation => nav_bindings(),
        KeyContext::Command => vec![
            binding("escape", "Esc", "Close command line", None),
            binding("submit", "Enter", "Execute command", None),
        ],
    }
}

fn helix_bindings(context: KeyContext) -> Vec<BindingSpec> {
    match context {
        KeyContext::Global => vec![
            binding("open_command", ":", "Open command palette", Some(Action::OpenCommandPalette)),
            binding("help", "?", "Show which-key/help", Some(Action::Help)),
            binding("quit", "q", "Quit", Some(Action::Quit)),
            binding("next_view", "Tab", "Next view", Some(Action::NextView)),
            binding("prev_view", "Shift+Tab", "Previous view", Some(Action::PrevView)),
            binding("view_branches", "1", "Open branches panel", Some(Action::ShowBranchPanel)),
            binding("view_log", "2", "Open log panel", Some(Action::ShowLogPanel)),
            binding("view_stash", "4", "Open stash/shelve panel", Some(Action::ShowStashPanel)),
            binding("view_remote", "R", "Open remotes panel", Some(Action::ShowRemotePanel)),
            binding("view_shelve", "S", "Open shelves panel", Some(Action::ShowShelvePanel)),
        ],
        KeyContext::Main => vec![
            binding("stage", "s", "Stage selected file", Some(Action::Stage)),
            binding("stage_all", "S", "Stage all files", Some(Action::StageAll)),
            binding("unstage", "u", "Unstage selected file", Some(Action::Unstage)),
            binding("unstage_all", "U", "Unstage all files", Some(Action::UnstageAll)),
            binding("discard", "d", "Discard selected file", Some(Action::Discard)),
            binding("commit", "c", "Open commit dialog", Some(Action::CommitDialog)),
            binding("open_diff", "Enter", "Open diff popup", None),
            binding("reset_dialog", "Ctrl+u", "Open reset dialog", Some(Action::ResetDialog("mixed".to_string()))),
        ],
        _ => vim_bindings(context),
    }
}

fn nav_bindings() -> Vec<BindingSpec> {
    vec![
        binding("nav_up", "k", "Move up", None),
        binding("nav_down", "j", "Move down", None),
        binding("nav_top", "g", "Jump to top", None),
        binding("nav_bottom", "G", "Jump to bottom", None),
        binding("nav_page_up", "Ctrl+u", "Page up", None),
        binding("nav_page_down", "Ctrl+d", "Page down", None),
        binding("nav_up_alt", "Up", "Move up", None),
        binding("nav_down_alt", "Down", "Move down", None),
        binding("nav_page_up_alt", "PageUp", "Page up", None),
        binding("nav_page_down_alt", "PageDown", "Page down", None),
    ]
}

fn binding(id: &'static str, key: &str, description: &'static str, action: Option<Action>) -> BindingSpec {
    BindingSpec {
        id,
        key: key.to_string(),
        description,
        action,
    }
}

fn key_label(key: KeyEvent) -> String {
    match key.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            format!("Ctrl+{}", c.to_ascii_lowercase())
        }
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                "Shift+Tab".to_string()
            } else {
                "Tab".to_string()
            }
        }
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        _ => format!("{:?}", key.code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn helix_global_uses_colon_for_command_palette() {
        let cfg = CogitConfig { keymap: crate::config::KeymapConfig { preset: KeymapPreset::Helix, overrides: KeymapOverrides::default() } };
        let km = KeymapManager::new(&cfg);
        let action = km.resolve(KeyContext::Global, KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        assert!(matches!(action, Some(Action::OpenCommandPalette)));
    }

    #[test]
    fn vim_global_uses_colon_for_command_palette() {
        let cfg = CogitConfig::default();
        let km = KeymapManager::new(&cfg);
        let action = km.resolve(KeyContext::Global, KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        assert!(matches!(action, Some(Action::OpenCommandPalette)));
    }

    #[test]
    fn vim_main_space_toggles_stage() {
        let cfg = CogitConfig::default();
        let km = KeymapManager::new(&cfg);
        let action = km.resolve(KeyContext::Main, KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(matches!(action, Some(Action::ToggleStage)));
    }

    #[test]
    fn hint_only_bindings_are_skipped() {
        let cfg = CogitConfig::default();
        let km = KeymapManager::new(&cfg);
        // open_diff is hint-only (None) in Main — Enter should not resolve to anything
        let action = km.resolve(KeyContext::Main, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(action.is_none());
    }

    #[test]
    fn vim_global_shelve_on_w() {
        let cfg = CogitConfig::default();
        let km = KeymapManager::new(&cfg);
        let action = km.resolve(KeyContext::Global, KeyEvent::new(KeyCode::Char('W'), KeyModifiers::SHIFT));
        assert!(matches!(action, Some(Action::ShowShelvePanel)));
    }
}
