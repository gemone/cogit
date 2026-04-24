use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
}

impl KeyContext {
    const ALL: [KeyContext; 7] = [
        KeyContext::Global,
        KeyContext::Main,
        KeyContext::Branches,
        KeyContext::Log,
        KeyContext::Stash,
        KeyContext::Remote,
        KeyContext::Shelve,
    ];

    /// Override map key used in KeymapOverrides.views
    fn override_key(self) -> Option<&'static str> {
        match self {
            Self::Global => None,
            Self::Main => Some("main"),
            Self::Branches => Some("branches"),
            Self::Log => Some("log"),
            Self::Stash => Some("stash"),
            Self::Remote => Some("remote"),
            Self::Shelve => Some("shelve"),
        }
    }
}

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

/// Pre-computed lookup for a single context.
#[derive(Debug, Clone)]
struct ContextCache {
    /// Ordered specs for which-key display.
    specs: Vec<BindingSpec>,
    /// key label → action for fast resolve.
    lookup: HashMap<String, Action>,
}

#[derive(Debug, Clone)]
struct KeymapState {
    preset: KeymapPreset,
    overrides: KeymapOverrides,
    cache: HashMap<KeyContext, ContextCache>,
}

impl KeymapState {
    fn rebuild_cache(&mut self) {
        self.cache.clear();
        for ctx in KeyContext::ALL {
            let specs = build_bindings(self.preset, ctx, &self.overrides);
            let lookup = specs.iter()
                .filter_map(|s| s.action.clone().map(|a| (s.key.clone(), a)))
                .collect();
            self.cache.insert(ctx, ContextCache { specs, lookup });
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeymapManager {
    state: Arc<RwLock<KeymapState>>,
}

impl KeymapManager {
    pub fn new(config: &CogitConfig) -> Self {
        let mut state = KeymapState {
            preset: config.keymap.preset,
            overrides: config.keymap.overrides.clone(),
            cache: HashMap::new(),
        };
        state.rebuild_cache();
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub fn preset(&self) -> KeymapPreset {
        self.state.read().expect("keymap lock poisoned").preset
    }

    pub fn set_preset(&self, preset: KeymapPreset) {
        let mut state = self.state.write().expect("keymap lock poisoned");
        state.preset = preset;
        state.rebuild_cache();
    }

    pub fn set_overrides(&self, overrides: KeymapOverrides) {
        let mut state = self.state.write().expect("keymap lock poisoned");
        state.overrides = overrides;
        state.rebuild_cache();
    }

    pub fn preset_name(&self) -> &'static str {
        self.preset().as_str()
    }

    /// Resolve a key to an action. Checks context bindings first, then Global.
    /// Skips hint-only bindings (action: None) so they don't block further search.
    pub fn resolve(&self, context: KeyContext, key: KeyEvent) -> Option<Action> {
        let pressed = key_label(key);
        let state = self.state.read().expect("keymap lock poisoned");

        // Check context first
        if let Some(ctx) = state.cache.get(&context) {
            if let Some(action) = ctx.lookup.get(&pressed).cloned() {
                return Some(action);
            }
        }

        // Fallback to Global (skip if already Global)
        if context != KeyContext::Global {
            if let Some(global) = state.cache.get(&KeyContext::Global) {
                if let Some(action) = global.lookup.get(&pressed).cloned() {
                    return Some(action);
                }
            }
        }

        None
    }

    pub fn bindings_for(&self, context: KeyContext) -> Vec<KeyBindingHint> {
        let state = self.state.read().expect("keymap lock poisoned");
        state.cache.get(&context)
            .map(|ctx| ctx.specs.iter()
                .map(|spec| KeyBindingHint { key: spec.key.clone(), description: spec.description })
                .collect())
            .unwrap_or_default()
    }

    pub fn override_binding(&self, context: KeyContext, action_id: &str, key: String) {
        let mut state = self.state.write().expect("keymap lock poisoned");
        if let Some(k) = context.override_key() {
            state.overrides.views.entry(k.to_string()).or_default().insert(action_id.to_string(), key);
        } else {
            state.overrides.global.insert(action_id.to_string(), key);
        }
        state.rebuild_cache();
    }
}

fn build_bindings(preset: KeymapPreset, context: KeyContext, overrides: &KeymapOverrides) -> Vec<BindingSpec> {
    let mut defaults = match preset {
        KeymapPreset::Vim => vim_bindings(context),
        KeymapPreset::Helix => helix_bindings(context),
    };

    let override_map = match context.override_key() {
        None => Some(&overrides.global),
        Some(k) => overrides.views.get(k),
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
            binding("stage_all", "A", "Stage all files", Some(Action::StageAll)),
            binding("unstage", "u", "Unstage selected file", Some(Action::Unstage)),
            binding("unstage_all", "V", "Unstage all files", Some(Action::UnstageAll)),
            binding("discard", "d", "Discard selected file", Some(Action::Discard)),
            binding("commit", "c", "Open commit dialog", Some(Action::CommitDialog)),
            binding("open_diff", "Enter", "Open diff popup", None),
            binding("reset_dialog", "Ctrl+u", "Open reset dialog", Some(Action::ResetDialog("mixed".to_string()))),
        ],
        _ => vim_bindings(context),
    }
}

fn binding(id: &'static str, key: &str, description: &'static str, action: Option<Action>) -> BindingSpec {
    BindingSpec { id, key: key.to_string(), description, action }
}

fn key_label(key: KeyEvent) -> String {
    match key.code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            format!("Ctrl+{}", c.to_ascii_lowercase())
        }
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::BackTab => "Shift+Tab".into(),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => "Shift+Tab".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        _ => format!("{:?}", key.code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn vim_km() -> KeymapManager {
        KeymapManager::new(&CogitConfig::default())
    }

    fn helix_km() -> KeymapManager {
        KeymapManager::new(&CogitConfig {
            keymap: crate::config::KeymapConfig { preset: KeymapPreset::Helix, overrides: KeymapOverrides::default() },
        })
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn helix_global_colon_opens_command_palette() {
        assert!(matches!(helix_km().resolve(KeyContext::Global, key(KeyCode::Char(':'))), Some(Action::OpenCommandPalette)));
    }

    #[test]
    fn vim_global_colon_opens_command_palette() {
        assert!(matches!(vim_km().resolve(KeyContext::Global, key(KeyCode::Char(':'))), Some(Action::OpenCommandPalette)));
    }

    #[test]
    fn vim_main_space_toggles_stage() {
        assert!(matches!(vim_km().resolve(KeyContext::Main, key(KeyCode::Char(' '))), Some(Action::ToggleStage)));
    }

    #[test]
    fn hint_only_bindings_skipped() {
        // open_diff is hint-only (None) — Enter should not resolve
        assert!(vim_km().resolve(KeyContext::Main, key(KeyCode::Enter)).is_none());
    }

    #[test]
    fn vim_global_w_opens_shelve() {
        assert!(matches!(
            vim_km().resolve(KeyContext::Global, KeyEvent::new(KeyCode::Char('W'), KeyModifiers::SHIFT)),
            Some(Action::ShowShelvePanel)
        ));
    }

    #[test]
    fn vim_main_s_vs_shift_s_case_sensitive() {
        assert!(matches!(
            vim_km().resolve(KeyContext::Main, key(KeyCode::Char('s'))),
            Some(Action::Stage)
        ));
        assert!(matches!(
            vim_km().resolve(KeyContext::Main, KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT)),
            Some(Action::StageAll)
        ));
    }

    #[test]
    fn vim_main_u_vs_shift_u_case_sensitive() {
        assert!(matches!(
            vim_km().resolve(KeyContext::Main, key(KeyCode::Char('u'))),
            Some(Action::Unstage)
        ));
        assert!(matches!(
            vim_km().resolve(KeyContext::Main, KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT)),
            Some(Action::UnstageAll)
        ));
    }

    #[test]
    fn set_preset_rebuilds_cache() {
        let km = vim_km();
        assert!(km.resolve(KeyContext::Global, key(KeyCode::Tab)).is_none());
        km.set_preset(KeymapPreset::Helix);
        assert!(matches!(km.resolve(KeyContext::Global, key(KeyCode::Tab)), Some(Action::NextView)));
    }
}
