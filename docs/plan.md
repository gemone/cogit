# Cogit — IDEA-Style Git TUI (Rust) Implementation Plan

> **Goal:** Build a vim-centric, JetBrains IDEA-inspired Git repository TUI in Rust using `git2` (libgit2), `ratatui`, and `crossterm`.

**Architecture:** Multi-panel layout mimicking IDEA's Git tool window: left sidebar (branches/remotes/tags/stashes/shelves), center file list (unstaged/staged/conflicted), bottom diff viewer, top command bar. All navigation uses vim keybindings; operations follow IDEA's mental model (shelve, cherry-pick, rebase flow, ignore patterns).

**Tech Stack:** Rust 2024 edition, `git2` (libgit2 bindings), `ratatui` + `crossterm`, `serde` + `config`, `anyhow` + `thiserror`.

---

## Crate Layout

```
cogit/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry (clap), init logger, run App
│   ├── app.rs               # TUI app: root component, panel orchestration, event loop
│   ├── app/
│   │   ├── keymap.rs        # Vim keymap definitions + user overrides
│   │   ├── styles.rs        # ratatui color themes (dark/light/high-contrast)
│   │   ├── help.rs          # Context-aware help overlay
│   │   └── cmdline.rs       # Vim-style command palette (`:`)
│   ├── panels/
│   │   ├── mod.rs           # Panel trait (focus, render, handle_event)
│   │   ├── sidebar.rs       # Left: ref tree (HEAD, branches, remotes, tags, stashes, shelves)
│   │   ├── filelist.rs      # Center: working tree file status list
│   │   ├── diffviewer.rs    # Bottom/Right: unified diff with hunk navigation
│   │   └── cmdbar.rs        # Top: breadcrumbs, operation hints, mode indicator
│   ├── gitops/
│   │   ├── mod.rs           # Repo wrapper, error types
│   │   ├── repo.rs          # Open/close repository, head shorthand
│   │   ├── status.rs        # Status iteration → FileStatus structs
│   │   ├── branch.rs        # Branch CRUD, checkout, tracking
│   │   ├── commit.rs        # Commit, amend, cherry-pick
│   │   ├── merge_rebase.rs  # Merge, rebase (abort/continue/skip), pull
│   │   ├── stash.rs         # Stash push/pop/list/apply/drop
│   │   ├── remote.rs        # Fetch, push, remote list
│   │   ├── ignore.rs        # .gitignore read/write
│   │   └── shelve.rs        # IDEA-style shelve (patch files in .cogit/shelves/)
│   ├── vimkeys.rs           # Vim motion parser (counts, gg/G, C-d/C-u, /search)
│   └── config.rs            # CogitConfig: keymap overrides, theme, editor
```

---

## Phase Overview

| Phase | Focus | Est. Files | Est. Lines | Risk |
|---|---|---|---|---|
| P1 | Foundation: CLI, config, git2 repo wrapper, status | 8 | ~700 | git2 build time |
| P2 | Core UI: App shell, panel trait, vim keymap, styles | 7 | ~800 | panel focus mgmt |
| P3 | File/Diff Center: file list, diff viewer, staging | 5 | ~700 | diff rendering perf |
| P4 | Branch & Remote: checkout, merge, rebase, pull, fetch | 6 | ~700 | rebase state detection |
| P5 | IDEA Extras: shelve, ignore manager, stash, cherry-pick | 5 | ~600 | patch file I/O |
| P6 | Polish: command palette, help, search/filter, edge cases | 4 | ~400 | — |
| **Total** | | **35** | **~3900** | |

---

## P1: Foundation

### Task 1.1: CLI & Config

- **Create:** `src/config.rs`
  - `struct CogitConfig { editor: String, theme: Theme, keymap_overrides: HashMap<...> }`
  - Load from `~/.config/cogit/config.toml` via `config` crate
- **Create:** `src/main.rs`
  - `clap` CLI: `--repo`/`-C`, `--version`
  - Default repo = current dir
  - `anyhow::Result<()>` main
- **Create:** `Cargo.toml` deps

### Task 1.2: git2 Repo Wrapper

- **Create:** `src/gitops/mod.rs`
  - `pub use` all gitops modules; define `GitError` via `thiserror`
- **Create:** `src/gitops/repo.rs`
  - `pub struct Repo { inner: git2::Repository, path: PathBuf }`
  - `Repo::open(path: &Path) -> Result<Self>`
  - `head_shorthand(&self) -> Option<String>`
- **Create:** `src/gitops/status.rs`
  - `enum FileStatus { Untracked, Modified, StagedNew, StagedModified, Conflicted, Ignored }`
  - `struct WorktreeFile { path: String, status: FileStatus }`
  - `Repo::status(&self) -> Result<Vec<WorktreeFile>>`

---

## P2: Core UI Shell

### Task 2.1: Panel Abstraction

- **Create:** `src/panels/mod.rs`
  - `trait Panel { fn focus(&mut self); fn blur(&mut self); fn render(&self, area: Rect, buf: &mut Buffer); fn handle_key(&mut self, key: KeyEvent) -> Option<Action>; fn title(&self) -> &str; }`

### Task 2.2: Vim Key Engine

- **Create:** `src/vimkeys.rs`
  - Parse `crossterm::event::KeyEvent` into vim motions
  - Normal mode: `h/j/k/l`, `gg`, `G`, `C-d`, `C-u`, `0`, `$`, `w`, `b`, `e`
  - Counts: `5j`, `10k`, `20gg`
  - Search: `/pattern`, `n`, `N`
  - Visual: `v`, `V`, `C-v` (block)
  - Actions: `:`, `?`, `q`
  - Returns `enum Motion { Up(usize), Down(usize), Top, Bottom, ... }`

### Task 2.3: App Shell & Layout

- **Create:** `src/app.rs`
  - `struct App { repo: Repo, panels: Vec<Box<dyn Panel>>, focus_idx: usize, mode: Mode, size: Rect, should_quit: bool, cmdline: Option<Cmdline>, help: Option<Help> }`
  - `impl App { fn new(repo: Repo) -> Self; fn run(&mut self, terminal: &mut Terminal) -> Result<()>; fn draw(&self, terminal: &mut Terminal)?; }`
  - Layout: `Constraint::Length(1)` top bar + `Constraint::Min(0)` middle (sidebar | filelist | diff) + `Constraint::Length(1)` bottom status or `Constraint::Length(3)` cmdline
  - Middle split: 25% | 35% | 40%
- **Create:** `src/app/styles.rs`
  - `enum Theme { Dark, Light, HighContrast }`
  - `struct Styles { border_active: Style, border_inactive: Style, addition: Style, deletion: Style, context: Style, conflict: Style, header: Style, ... }`
- **Create:** `src/app/keymap.rs`
  - `struct KeyMap { global: HashMap<KeyCombo, Action>, filelist: HashMap<...>, diff: ..., sidebar: ... }`
  - `enum Action { FocusSidebar, FocusFilelist, FocusDiff, Stage, Unstage, StageAll, UnstageAll, Discard, ToggleIgnore, ToggleUntracked, Checkout, NewBranch, DeleteBranch, Refresh, Quit, CommandPalette, Help, ... }`

---

## P3: File/Diff Center (IDEA "Local Changes")

### Task 3.1: File List Panel

- **Create:** `src/panels/filelist.rs`
  - `struct FileListPanel { items: Vec<WorktreeFile>, cursor: usize, offset: usize, show_ignored: bool, show_untracked: bool, staged_filter: Option<bool> }`
  - Render as `ratatui::widgets::Table` or `List` with sections: Conflicted → Changes → Staged
  - Vim nav: `j/k`, `Space` toggle stage/unstage, `s` stage, `u` unstage, `d` discard (confirm modal), `Enter` open diff
  - `a` stage all, `A` unstage all, `i` ignore, `I` toggle ignored, `U` toggle untracked, `/` search

### Task 3.2: Diff Viewer Panel

- **Create:** `src/panels/diffviewer.rs`
  - `struct DiffViewerPanel { diff_text: Vec<DiffLine>, cursor: usize, offset: usize, hunks: Vec<Hunk> }`
  - Unified diff with line colors: addition green, deletion red, context gray, header yellow
  - Vim nav: `j/k` line, `]c`/`[c` next/prev hunk
  - `s` stage hunk, `u` unstage hunk (if supported by git2 line-wise)

### Task 3.3: Sidebar Panel

- **Create:** `src/panels/sidebar.rs`
  - `struct SidebarPanel { tree: TreeNode, cursor: usize, expanded: HashSet<String> }`
  - Tree sections: HEAD, Local Branches, Remote Branches, Tags, Remotes, Stashes, Shelves
  - Vim nav: `j/k`, `o`/`Enter` expand/collapse, `c` checkout, `b` create branch, `D` delete (confirm), `r` refresh/fetch

---

## P4: Branch & Remote Operations

### Task 4.1: Branch Operations

- **Create:** `src/gitops/branch.rs`
  - `Repo::branches(&self) -> Result<Vec<BranchInfo>>`
  - `Repo::checkout(&self, name: &str) -> Result<()>`
  - `Repo::create_branch(&self, name: &str, base: &str) -> Result<()>`
  - `Repo::delete_branch(&self, name: &str, force: bool) -> Result<()>`
  - `Repo::set_upstream(&self, local: &str, remote: &str) -> Result<()>`

### Task 4.2: Merge & Rebase

- **Create:** `src/gitops/merge_rebase.rs`
  - `Repo::merge(&self, branch: &str) -> Result<bool>` (returns has_conflicts)
  - `Repo::rebase(&self, branch: &str) -> Result<()>`
  - `Repo::rebase_continue(&self)`, `rebase_abort(&self)`, `rebase_skip(&self)`
  - `Repo::pull(&self, remote: &str, branch: &str) -> Result<()>` (fetch + merge/rebase)
  - Detect rebase/merge state via git2 `Repository::state()`

### Task 4.3: Remote Operations

- **Create:** `src/gitops/remote.rs`
  - `Repo::fetch(&self, remote: &str) -> Result<()>`
  - `Repo::push(&self, remote: &str, refspec: &str) -> Result<()>`
  - `Repo::remotes(&self) -> Result<Vec<String>>`

---

## P5: IDEA Extras

### Task 5.1: Shelve (IDEA-style)

- **Create:** `src/gitops/shelve.rs`
  - Store patches in `.cogit/shelves/` as `.patch` files (outside git objects, user-visible)
  - `Repo::shelve(&self, name: &str, paths: &[&str]) -> Result<()>` — generate unified diff, write to `.cogit/shelves/{name}.patch`, then checkout those paths to reset them
  - `Repo::unshelve(&self, name: &str) -> Result<()>` — apply patch via `git2::ApplyOptions`
  - `Repo::delete_shelve(&self, name: &str) -> Result<()>`
  - `Repo::list_shelves(&self) -> Result<Vec<ShelveInfo>>`
  - UI: sidebar "Shelves" tree, `N` new shelve from current changes, `Enter` unshelve, `D` delete

### Task 5.2: Ignore Manager

- **Create:** `src/gitops/ignore.rs`
  - `Repo::ignore_patterns(&self, dir: &Path) -> Result<Vec<String>>`
  - `Repo::add_ignore(&self, dir: &Path, pattern: &str) -> Result<()>`
  - `Repo::remove_ignore(&self, dir: &Path, pattern: &str) -> Result<()>`
  - UI: select file → `i` opens prompt with suggested pattern (`*.log`, `/build/`), choose scope (root/subdir)

### Task 5.3: Stash

- **Create:** `src/gitops/stash.rs`
  - `Repo::stash_save(&self, msg: &str, include_untracked: bool) -> Result<()>`
  - `Repo::stash_pop(&self, index: usize) -> Result<()>`
  - `Repo::stash_apply(&self, index: usize) -> Result<()>`
  - `Repo::stash_drop(&self, index: usize) -> Result<()>`
  - `Repo::stash_list(&self) -> Result<Vec<StashEntry>>`
  - UI: sidebar under Stashes tree

### Task 5.4: Cherry-pick

- **Create:** `src/gitops/commit.rs`
  - `Repo::cherry_pick(&self, oid: &str) -> Result<()>`
  - `Repo::cherry_pick_abort(&self)`, `cherry_pick_continue(&self)`
  - `Repo::commit(&self, message: &str) -> Result<()>`
  - `Repo::commit_amend(&self, message: &str) -> Result<()>`

---

## P6: Polish

### Task 6.1: Command Palette

- **Create:** `src/app/cmdline.rs`
  - `struct Cmdline { input: String, cursor: usize, suggestions: Vec<String> }`
  - Render at bottom like vim command line
  - Commands: `:q`, `:q!`, `:w`, `:wq`, `:commit`, `:merge <branch>`, `:rebase <branch>`, `:fetch`, `:push`, `:pull`, `:stash`, `:shelve <name>`, `:unshelve <name>`, `:branch <name>`
  - Tab completion

### Task 6.2: Help System

- **Create:** `src/app/help.rs`
  - `?` toggles full-screen overlay
  - Context-aware: shows keybindings for current panel + current mode (Normal/Visual/Command)

### Task 6.3: Theming & Edge Cases

- **Create:** `src/app/keymap.rs` override loading from config
- Error modal popup (anyhow error display)
- Confirm modal for destructive actions (`d`, `D`, `:q!`)
- Async status refresh on file system changes (optional, not MVP)

---

## Vim Keybinding Reference (User-Facing)

| Context | Key | Action |
|---|---|---|
| **Global** | `1` | Focus Sidebar |
| | `2` | Focus Filelist |
| | `3` | Focus Diff |
| | `Tab` / `S-Tab` | Next / Prev panel |
| | `:` | Command palette |
| | `?` | Toggle help |
| | `q` / `:q` | Quit |
| **Sidebar** | `j/k` | Move up/down |
| | `o/Enter` | Expand/collapse tree |
| | `c` | Checkout branch |
| | `b` | New branch (prompt) |
| | `D` | Delete branch/tag/stash/shelve |
| | `g` / `G` | Top / Bottom |
| | `r` | Refresh / fetch remotes |
| **Filelist** | `j/k` | Move |
| | `Space` | Toggle stage/unstage |
| | `s` | Stage selected |
| | `u` | Unstage selected |
| | `a` | Stage all |
| | `A` | Unstage all |
| | `d` | Discard changes (confirm) |
| | `Enter` | Open diff |
| | `i` | Add to .gitignore (prompt) |
| | `I` | Toggle show ignored |
| | `U` | Toggle show untracked |
| | `/` | Search files |
| **Diff** | `j/k` | Line up/down |
| | `]c` / `[c` | Next / Prev hunk |
| | `s` | Stage hunk |
| | `u` | Unstage hunk |
| | `d` | Reset hunk |
| **Command** | `:w` | Commit (opens editor) |
| | `:q` | Close panel / quit |
| | `:e <file>` | External editor |
| | `:merge <branch>` | Merge |
| | `:rebase <branch>` | Rebase |
| | `:pull` | Pull |
| | `:push` | Push |
| | `:stash` | Stash current |
| | `:shelve <name>` | Shelve changes |
| | `:unshelve <name>` | Unshelve |

---

## git2 / libgit2 Notes

- Crate: `git2 = "0.20"`
- On macOS/Linux, `git2` builds a bundled libgit2 automatically on first compile.
- Build may take 2–5 minutes for the initial `cargo build` due to C compilation.
- For diff: use `git2::Diff` APIs to generate unified diff text, then parse into lines/hunks in Rust.
- For apply/shelve: use `git2::ApplyOptions` to apply generated patch strings.

---

## Risks & Mitigations

1. **git2 build time:** Initial compile compiles libgit2 C sources. Mitigation: accept the wait; subsequent builds are fast.
2. **Rebase/merge conflict UI:** Complex state machine. Mitigation: detect state via `repo.state()`; show clear mode indicator in cmdbar.
3. **Performance on large repos:** Status/diff may be slow. Mitigation: virtual scrolling in lists; limit diff to first N lines until scrolled.
4. **Windows compatibility:** libgit2 + crossterm both support Windows, but path handling needs testing. Initial target macOS/Linux.

---

## Next Step

Verify `cargo build` passes → commit plan → delegate to opencode ACP in batches (P1+P2, P3, P4+P5, P6).
