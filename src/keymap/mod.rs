//! Keymap: per-context bindings from `KeySpec` to `ActionName`, the default
//! tables (mirroring the current hardcoded behavior exactly), and loading /
//! merging of the optional user config at `~/.config/cogit/keymap.toml`.

mod action_name;
mod key;

pub use action_name::{ActionName, action_from_name};
pub use key::KeySpec;

use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

/// UI contexts that have their own key binding namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    Main,
    Branches,
    Log,
    Stash,
    Shelve,
    Remote,
    Global,
}

impl Context {
    /// The context matching the current `View` (Global is never returned).
    pub fn from_view_name(name: &str) -> Option<Context> {
        match name {
            "Main" => Some(Context::Main),
            "Branches" => Some(Context::Branches),
            "Log" => Some(Context::Log),
            "Stash" => Some(Context::Stash),
            "Shelve" => Some(Context::Shelve),
            "Remote" => Some(Context::Remote),
            "Global" => Some(Context::Global),
            _ => None,
        }
    }
}

/// The application keymap: (context, key) -> action name.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<(Context, KeySpec), ActionNameBuf>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl Keymap {
    /// Build a keymap from explicit (context, key, action) triples.
    pub fn from_bindings(bindings: Vec<(Context, KeySpec, ActionName)>) -> Self {
        Self {
            bindings: bindings
                .into_iter()
                .map(|(c, k, a)| ((c, k), ActionNameBuf::from(a)))
                .collect(),
        }
    }

    /// The default keymap, identical to the pre-keymap hardcoded behavior.
    pub fn with_defaults() -> Self {
        use Context as C;
        use KeyCode::{Char, Down, Enter, Esc, PageDown, PageUp, Tab, Up};
        let m = |mods: KeyModifiers| mods;
        Self::from_bindings(vec![
            // ---- Global ----
            (
                C::Global,
                KeySpec::new(Char('?'), KeyModifiers::NONE),
                "help",
            ),
            // ---- Main (file list) ----
            (
                C::Main,
                KeySpec::new(Char('j'), KeyModifiers::NONE),
                "nav_down",
            ),
            (C::Main, KeySpec::new(Down, KeyModifiers::NONE), "nav_down"),
            (
                C::Main,
                KeySpec::new(Char('k'), KeyModifiers::NONE),
                "nav_up",
            ),
            (C::Main, KeySpec::new(Up, KeyModifiers::NONE), "nav_up"),
            (
                C::Main,
                KeySpec::new(Char('G'), m(KeyModifiers::SHIFT)),
                "nav_bottom",
            ),
            (
                C::Main,
                KeySpec::new(Char('g'), KeyModifiers::NONE),
                "nav_top",
            ),
            (
                C::Main,
                KeySpec::new(PageDown, KeyModifiers::NONE),
                "nav_page_down",
            ),
            (
                C::Main,
                KeySpec::new(Char('J'), m(KeyModifiers::SHIFT)),
                "nav_page_down",
            ),
            (
                C::Main,
                KeySpec::new(PageUp, KeyModifiers::NONE),
                "nav_page_up",
            ),
            (
                C::Main,
                KeySpec::new(Char('K'), m(KeyModifiers::SHIFT)),
                "nav_page_up",
            ),
            (
                C::Main,
                KeySpec::new(Char('u'), KeyModifiers::CONTROL),
                "nav_page_up",
            ),
            (
                C::Main,
                KeySpec::new(Char('d'), KeyModifiers::CONTROL),
                "nav_page_down",
            ),
            (C::Main, KeySpec::new(Char('q'), KeyModifiers::NONE), "quit"),
            (
                C::Main,
                KeySpec::new(Char('s'), KeyModifiers::NONE),
                "stage",
            ),
            (
                C::Main,
                KeySpec::new(Char('A'), m(KeyModifiers::SHIFT)),
                "stage_all",
            ),
            (
                C::Main,
                KeySpec::new(Char('u'), KeyModifiers::NONE),
                "unstage",
            ),
            (
                C::Main,
                KeySpec::new(Char('U'), m(KeyModifiers::SHIFT)),
                "unstage_all",
            ),
            (
                C::Main,
                KeySpec::new(Char('c'), KeyModifiers::NONE),
                "commit_dialog",
            ),
            (
                C::Main,
                KeySpec::new(Char(' '), KeyModifiers::NONE),
                "stage_toggle",
            ),
            (C::Main, KeySpec::new(Enter, KeyModifiers::NONE), "diff"),
            (
                C::Main,
                KeySpec::new(Char('1'), KeyModifiers::NONE),
                "show_branches",
            ),
            (
                C::Main,
                KeySpec::new(Char('2'), KeyModifiers::NONE),
                "show_log",
            ),
            (
                C::Main,
                KeySpec::new(Char('4'), KeyModifiers::NONE),
                "show_stash",
            ),
            (
                C::Main,
                KeySpec::new(Char('R'), m(KeyModifiers::SHIFT)),
                "show_remote",
            ),
            (
                C::Main,
                KeySpec::new(Char('S'), m(KeyModifiers::SHIFT)),
                "show_shelve",
            ),
            (
                C::Main,
                KeySpec::new(Char(':'), KeyModifiers::NONE),
                "command_line",
            ),
            // ---- Branches ----
            (
                C::Branches,
                KeySpec::new(Char('q'), KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Branches,
                KeySpec::new(Esc, KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Branches,
                KeySpec::new(Char('n'), KeyModifiers::NONE),
                "branch_create_dialog",
            ),
            (
                C::Branches,
                KeySpec::new(Char('R'), m(KeyModifiers::SHIFT)),
                "branch_rename_dialog",
            ),
            (
                C::Branches,
                KeySpec::new(Char('d'), KeyModifiers::NONE),
                "branch_delete",
            ),
            (
                C::Branches,
                KeySpec::new(Char('f'), KeyModifiers::NONE),
                "fetch_all",
            ),
            (
                C::Branches,
                KeySpec::new(Char('p'), KeyModifiers::NONE),
                "push_current",
            ),
            (
                C::Branches,
                KeySpec::new(Char('P'), m(KeyModifiers::SHIFT)),
                "pull_current",
            ),
            (
                C::Branches,
                KeySpec::new(Char('m'), KeyModifiers::NONE),
                "branch_merge",
            ),
            (
                C::Branches,
                KeySpec::new(Char('r'), KeyModifiers::NONE),
                "branch_rebase",
            ),
            (
                C::Branches,
                KeySpec::new(Enter, KeyModifiers::NONE),
                "branch_checkout",
            ),
            // ---- Log ----
            (
                C::Log,
                KeySpec::new(Char('q'), KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Log,
                KeySpec::new(Esc, KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Log,
                KeySpec::new(Char('y'), KeyModifiers::NONE),
                "log_copy_hash",
            ),
            (
                C::Log,
                KeySpec::new(Char('c'), KeyModifiers::NONE),
                "log_cherry_pick",
            ),
            (
                C::Log,
                KeySpec::new(Char('/'), KeyModifiers::NONE),
                "log_search",
            ),
            // ---- Stash ----
            (
                C::Stash,
                KeySpec::new(Char('q'), KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Stash,
                KeySpec::new(Esc, KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Stash,
                KeySpec::new(Tab, KeyModifiers::NONE),
                "stash_tab_toggle",
            ),
            (
                C::Stash,
                KeySpec::new(Char('s'), KeyModifiers::NONE),
                "stash_create",
            ),
            (
                C::Stash,
                KeySpec::new(Enter, KeyModifiers::NONE),
                "stash_pop",
            ),
            (
                C::Stash,
                KeySpec::new(Char('a'), KeyModifiers::NONE),
                "stash_apply",
            ),
            (
                C::Stash,
                KeySpec::new(Char('d'), KeyModifiers::NONE),
                "stash_drop",
            ),
            // ---- Shelve ----
            (
                C::Shelve,
                KeySpec::new(Char('q'), KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Shelve,
                KeySpec::new(Esc, KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Shelve,
                KeySpec::new(Char('n'), KeyModifiers::NONE),
                "shelve_create",
            ),
            (
                C::Shelve,
                KeySpec::new(Char('s'), KeyModifiers::NONE),
                "shelve_toggle_staged",
            ),
            (
                C::Shelve,
                KeySpec::new(Char('p'), KeyModifiers::NONE),
                "shelve_apply",
            ),
            (
                C::Shelve,
                KeySpec::new(Char('a'), KeyModifiers::NONE),
                "shelve_apply_keep",
            ),
            (
                C::Shelve,
                KeySpec::new(Char('d'), KeyModifiers::NONE),
                "shelve_drop",
            ),
            (C::Shelve, KeySpec::new(Enter, KeyModifiers::NONE), "diff"),
            // ---- Remote ----
            (
                C::Remote,
                KeySpec::new(Char('q'), KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Remote,
                KeySpec::new(Esc, KeyModifiers::NONE),
                "back_to_main",
            ),
            (
                C::Remote,
                KeySpec::new(Char('a'), KeyModifiers::NONE),
                "remote_add",
            ),
            (
                C::Remote,
                KeySpec::new(Char('d'), KeyModifiers::NONE),
                "remote_remove",
            ),
            (
                C::Remote,
                KeySpec::new(Char('r'), KeyModifiers::NONE),
                "remote_rename",
            ),
            (
                C::Remote,
                KeySpec::new(Char('u'), KeyModifiers::NONE),
                "remote_fetch",
            ),
            (
                C::Remote,
                KeySpec::new(Enter, KeyModifiers::NONE),
                "remote_show_branches",
            ),
        ])
    }

    /// Look up the action bound to a key in a context.
    pub fn get(&self, context: Context, key: &KeySpec) -> Option<&str> {
        self.bindings.get(&(context, *key)).map(|a| a.0.as_str())
    }

    /// Insert or override a binding.
    pub fn set(&mut self, context: Context, key: KeySpec, action: ActionName) {
        self.bindings
            .insert((context, key), ActionNameBuf::from(action));
    }

    /// Number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// True if there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// Owned action name stored in the keymap (`String` newtype so user-supplied
/// names from TOML can be stored without leaking).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionNameBuf(String);

impl From<&'static str> for ActionNameBuf {
    fn from(s: &'static str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ActionNameBuf {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Serde shape of the user TOML file:
///
/// ```toml
/// [keys.Main]
/// "s" = "stage"
/// "Ctrl+a" = "stage_all"
///
/// [keys.Global]
/// "?" = "help"
/// ```
#[derive(Debug, Default, serde::Deserialize)]
struct KeymapFile {
    /// `context name -> (key spec string -> action name)`, e.g.
    /// `keys.Main."s" = "stage"`.
    keys: Option<HashMap<String, HashMap<String, String>>>,
}

/// Default path for the user keymap config: `$HOME/.config/cogit/keymap.toml`.
/// Uses std only (`$HOME` env var), avoiding extra dependencies.
pub fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| {
        std::path::PathBuf::from(home)
            .join(".config")
            .join("cogit")
            .join("keymap.toml")
    })
}

impl Keymap {
    /// Load the user keymap: defaults merged with `~/.config/cogit/keymap.toml`.
    /// Missing file -> pure defaults (no error); parse failure -> warn on
    /// stderr and use defaults.
    pub fn load() -> Self {
        Self::load_from_path(config_path().as_deref())
    }

    /// Like `load`, but reads the config from an explicit path (testable
    /// variant; production code always passes `None` -> `config_path()`).
    fn load_from_path(path: Option<&std::path::Path>) -> Self {
        let mut keymap = Self::with_defaults();
        let Some(path) = path else {
            return keymap;
        };
        match std::fs::read_to_string(path) {
            Err(_) => keymap, // missing or unreadable: pure defaults, no error
            Ok(content) => match toml::from_str::<KeymapFile>(&content) {
                Ok(file) => {
                    keymap.merge_toml_sections(file.keys.as_ref());
                    keymap
                }
                Err(e) => {
                    eprintln!(
                        "cogit: warning: failed to parse {}: {e}; using default keymap",
                        path.display()
                    );
                    keymap
                }
            },
        }
    }

    /// Apply user overrides from parsed TOML sections
    /// (`context name -> (key spec string -> action name)`).
    fn merge_toml_sections(&mut self, sections: Option<&HashMap<String, HashMap<String, String>>>) {
        let Some(sections) = sections else {
            return;
        };
        for (context_name, bindings) in sections {
            let Some(context) = Context::from_view_name(context_name) else {
                eprintln!(
                    "cogit: warning: unknown keymap context {context_name:?}; ignoring its bindings"
                );
                continue;
            };
            for (key_str, action_str) in bindings {
                match KeySpec::parse(key_str) {
                    Ok(spec) => {
                        self.bindings
                            .insert((context, spec), ActionNameBuf(action_str.clone()));
                    }
                    Err(e) => {
                        eprintln!(
                            "cogit: warning: invalid key {key_str:?} in [{context_name:?}] keymap config: {e}; ignoring"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> KeySpec {
        KeySpec::parse(s).unwrap()
    }

    #[test]
    fn default_main_matches_old_hardcoded_behavior() {
        let km = Keymap::with_defaults();
        // q quits, s stages, A stages all, c opens commit dialog
        assert_eq!(km.get(Context::Main, &k("q")), Some("quit"));
        assert_eq!(km.get(Context::Main, &k("s")), Some("stage"));
        assert_eq!(km.get(Context::Main, &k("A")), Some("stage_all"));
        assert_eq!(km.get(Context::Main, &k("u")), Some("unstage"));
        assert_eq!(km.get(Context::Main, &k("U")), Some("unstage_all"));
        assert_eq!(km.get(Context::Main, &k("c")), Some("commit_dialog"));
        assert_eq!(km.get(Context::Main, &k(":")), Some("command_line"));
        assert_eq!(km.get(Context::Main, &k("1")), Some("show_branches"));
        assert_eq!(km.get(Context::Main, &k("2")), Some("show_log"));
        assert_eq!(km.get(Context::Main, &k("4")), Some("show_stash"));
        assert_eq!(km.get(Context::Main, &k("R")), Some("show_remote"));
        assert_eq!(km.get(Context::Main, &k("S")), Some("show_shelve"));
        assert_eq!(
            km.get(
                Context::Main,
                &KeySpec::new(KeyCode::Char(' '), KeyModifiers::NONE)
            ),
            Some("stage_toggle")
        );
        assert_eq!(
            km.get(
                Context::Main,
                &KeySpec::new(KeyCode::Enter, KeyModifiers::NONE)
            ),
            Some("diff")
        );
    }

    #[test]
    fn default_global_help() {
        let km = Keymap::with_defaults();
        assert_eq!(km.get(Context::Global, &k("?")), Some("help"));
        assert_eq!(km.get(Context::Main, &k("?")), None);
    }

    #[test]
    fn default_panel_keys_registered() {
        let km = Keymap::with_defaults();
        assert_eq!(
            km.get(Context::Branches, &k("n")),
            Some("branch_create_dialog")
        );
        assert_eq!(km.get(Context::Branches, &k("d")), Some("branch_delete"));
        assert_eq!(km.get(Context::Log, &k("y")), Some("log_copy_hash"));
        assert_eq!(km.get(Context::Stash, &k("s")), Some("stash_create"));
        assert_eq!(km.get(Context::Shelve, &k("p")), Some("shelve_apply"));
        assert_eq!(km.get(Context::Remote, &k("a")), Some("remote_add"));
    }

    #[test]
    fn toml_merge_overrides_defaults() {
        let toml_str = r#"
[keys.Main]
"s" = "unstage"
"Ctrl+a" = "stage_all"
[keys.Global]
"?" = "quit"
"#;
        let file: KeymapFile = toml::from_str(toml_str).unwrap();
        let mut km = Keymap::with_defaults();
        km.merge_toml_sections(file.keys.as_ref());
        // Overridden
        assert_eq!(km.get(Context::Main, &k("s")), Some("unstage"));
        assert_eq!(km.get(Context::Main, &k("Ctrl+a")), Some("stage_all"));
        assert_eq!(km.get(Context::Global, &k("?")), Some("quit"));
        // Untouched defaults survive
        assert_eq!(km.get(Context::Main, &k("A")), Some("stage_all"));
        assert_eq!(km.get(Context::Main, &k("c")), Some("commit_dialog"));
    }

    #[test]
    fn toml_merge_ignores_unknown_context() {
        let toml_str = r#"
[keys.Nonsense]
"x" = "quit"
"#;
        let file: KeymapFile = toml::from_str(toml_str).unwrap();
        let mut km = Keymap::with_defaults();
        km.merge_toml_sections(file.keys.as_ref());
        assert_eq!(km.get(Context::Main, &k("s")), Some("stage"));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        // HOME pointing at an empty temp dir: no keymap.toml exists.
        let dir = std::env::temp_dir().join(format!("cogit-km-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let toml_path = dir.join(".config/cogit/keymap.toml");
        let km = Keymap::load_from_path(Some(&toml_path));
        assert_eq!(km.get(Context::Main, &k("s")), Some("stage"));
    }

    #[test]
    fn load_valid_toml_applies_overrides() {
        let dir = std::env::temp_dir().join(format!("cogit-km-test2-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".config/cogit")).unwrap();
        let toml_path = dir.join(".config/cogit/keymap.toml");
        std::fs::write(&toml_path, "[keys.Main]\n\"s\" = \"unstage\"\n").unwrap();
        let km = Keymap::load_from_path(Some(&toml_path));
        assert_eq!(km.get(Context::Main, &k("s")), Some("unstage"));
    }

    #[test]
    fn load_invalid_toml_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("cogit-km-test3-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".config/cogit")).unwrap();
        let toml_path = dir.join(".config/cogit/keymap.toml");
        std::fs::write(&toml_path, "[keys.Main\nbroken").unwrap();
        let km = Keymap::load_from_path(Some(&toml_path));
        assert_eq!(km.get(Context::Main, &k("s")), Some("stage"));
    }
}
