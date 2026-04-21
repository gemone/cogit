# Cogit — IDEA-Style Git TUI Implementation Plan

> **Goal:** Build a vim-centric, JetBrains IDEA-inspired Git repository TUI using Go + libgit2 (git2go) + Bubble Tea.

**Architecture:** Multi-panel layout mimicking IDEA's Git tool window: left sidebar (branches/remotes/tags), center file list (unstaged/staged/conflicted), bottom diff viewer, top command bar. All navigation uses vim keybindings; operations follow IDEA's mental model (shelve, cherry-pick, rebase flow, ignore patterns).

**Tech Stack:** Go 1.23+, git2go/v34 (libgit2), Bubble Tea + Lipgloss + Bubbles, cobra (CLI), Viper (config).

---

## Phase Overview

| Phase | Focus | Est. Files | Est. Lines | Risk |
|---|---|---|---|---|
| P1 | Foundation: repo init, git2go wrapper, config, CLI | 6 | ~600 | libgit2 CGO setup |
| P2 | Core UI: Bubble Tea app shell, panels, vim keymaps | 8 | ~900 | panel focus mgmt |
| P3 | File/Diff Center: status, stage/unstage, diff viewer | 6 | ~700 | diff rendering perf |
| P4 | Branch & Remote: checkout, create, merge, rebase, pull | 7 | ~800 | rebase conflict UI |
| P5 | IDEA Extras: shelve, ignore manager, stash, cherry-pick | 7 | ~700 | shelve patch format |
| P6 | Polish: help system, theming, search/filter, edge cases | 5 | ~400 | — |
| **Total** | | **39** | **~4100** | |

---

## Module & File Map

```
cogit/
├── cmd/
│   └── root.go              # cobra root command, --repo flag
├── internal/
│   ├── app/
│   │   ├── app.go           # Bubble Tea root model, panel orchestration
│   │   ├── keymap.go        # global & mode-specific vim keymaps
│   │   ├── styles.go        # lipgloss theme definitions
│   │   └── msg.go           # custom tea.Msg types
│   ├── panels/
│   │   ├── panel.go         # Panel interface (Focus/Blur/Update/View)
│   │   ├── sidebar.go       # left: refs (branches, remotes, tags)
│   │   ├── filelist.go      # center: working tree file status
│   │   ├── diffviewer.go    # bottom/right: unified diff with syntax-ish highlight
│   │   └── cmdbar.go        # top: breadcrumbs, pending operation hints
│   ├── gitops/
│   │   ├── repo.go          # git2go repository open/close wrapper
│   │   ├── status.go        # go-git2go status → our model
│   │   ├── branch.go        # branch CRUD, checkout, remote tracking
│   │   ├── commit.go        # commit, amend
│   │   ├── merge_rebase.go  # merge, rebase (abort/continue/skip), pull
│   │   ├── stash.go         # stash push/pop/list/apply
│   │   ├── remote.go        # fetch, push, remote list
│   │   ├── ignore.go        # .gitignore read/write/pattern validation
│   │   └── shelve.go        # IDEA-style shelve: patch files under .cogit/shelves/
│   ├── config/
│   │   └── config.go        # viper-based config: keymap overrides, theme, editor
│   └── vimkeys/
│       └── vim.go           # vim motion parser (gg/G, C-u/C-d, /search, etc.)
├── pkg/
│   └── diff/
│       └── diff.go          # minimal unified diff parser for hunk navigation
└── main.go
```

---

## P1: Foundation

### Task 1.1: CLI entrypoint & config

- **Create:** `cmd/root.go`
  - cobra root, `--repo`/`-C` flag, `--version`
  - default repo = current dir
- **Create:** `internal/config/config.go`
  - viper loads `~/.config/cogit/config.yaml`
  - struct: `Editor`, `Theme`, `KeymapOverrides`
- **Create:** `main.go`
- **Create:** `go.mod` deps (bubbletea, git2go/v34, cobra, viper)
- **Commit**

**Risk:** git2go/v34 requires libgit2 installed. Document `brew install libgit2` / `apt install libgit2-1.5-dev`.

### Task 1.2: git2go repo wrapper

- **Create:** `internal/gitops/repo.go`
  - `type Repo struct{ *git.Repository; path string }`
  - `Open(path string) (*Repo, error)`
  - `Close() error`
  - `Head() (string, error)` shorthand
- **Create:** `internal/gitops/status.go`
  - `FileStatus` enum: Untracked, Modified, StagedNew, StagedModified, Conflicted, Ignored
  - `Status() ([]FileStatus, error)` iterates git.StatusList
- **Test:** open `~/cogit` itself, print status.

---

## P2: Core UI Shell

### Task 2.1: Panel abstraction

- **Create:** `internal/panels/panel.go`
  - `type Panel interface { Focus() Panel; Blur() Panel; Update(tea.Msg) (Panel, tea.Cmd); View() string; Title() string }`
  - `type Base struct { focused bool; width, height int }`

### Task 2.2: Vim keymap engine

- **Create:** `internal/vimkeys/vim.go`
  - Normal mode: `h/j/k/l`, `gg`, `G`, `C-d`, `C-u`, `0`, `$`, `%`
  - Visual mode: `v`, `V`, `C-v` (block), `y`, `d`, `>`
  - Search: `/`, `n`, `N`
  - Action: `:` command palette, `?` help, `q` quit
  - Key represented as struct `{ Mod KeyMod; Key string }`
- **Create:** `internal/app/keymap.go`
  - Global bindings: `1-5` switch panels, `Tab` next panel, `S-Tab` prev
  - Mode enum: Normal, Visual, Insert (for commit msg), Command
  - `KeyMap` struct per panel + global
  - Supports user overrides from config

### Task 2.3: App shell & layout

- **Create:** `internal/app/app.go`
  - `type App struct { repo *gitops.Repo; panels []panels.Panel; focusIdx int; mode Mode; width, height int }`
  - `Init() tea.Cmd`: open repo, initial status load
  - `Update()`: route keys via keymap, resize msg propagates to panels
  - `View()`: layout = top bar (1 line) + middle (sidebar | filelist | [optional diff]) + bottom (diff or cmdline)
  - Default layout ratios: sidebar 25%, filelist 35%, diff 40%
- **Create:** `internal/app/styles.go`
  - Lipgloss styles: panel border active/inactive, file status colors, diff colors
- **Create:** `internal/app/msg.go`
  - `repoLoadedMsg`, `statusUpdatedMsg`, `errorMsg`, `commandPaletteOpenMsg`

---

## P3: File/Diff Center (IDEA "Local Changes" view)

### Task 3.1: File list panel

- **Create:** `internal/panels/filelist.go`
  - Tree view (flat for MVP) with sections: Conflicted → Changes (unstaged) → Staged
  - Columns: status icon, filename, dir path
  - Vim nav: `j/k`, `Space` toggle stage/unstage, `u` unstage, `s` stage, `d` discard (with confirm), `Enter` open diff
  - `a` stage all, `A` unstage all
  - Filter: `i` toggle ignored, `?` toggle untracked

### Task 3.2: Diff viewer panel

- **Create:** `internal/panels/diffviewer.go`
  - Unified diff view with hunk headers
  - Syntax-ish highlight: additions green, deletions red, context gray
  - Vim nav: `j/k` line, `]c`/`[c` next/prev hunk, `za` toggle fold hunk (MVP: no fold)
  - Stage/unstage individual lines/hunks: `s` stage hunk, `u` unstage hunk (IDEA-style)
  - `p` pick line (not MVP if libgit2 line staging is hard)

### Task 3.3: Sidebar panel (branches/remotes/tags)

- **Create:** `internal/panels/sidebar.go`
  - Tree sections: HEAD, Branches (local + remote), Tags, Remotes, Stashes, Shelves
  - Vim nav: `j/k`, `o`/`Enter` expand/collapse, `c` checkout branch, `b` create branch, `D` delete (with confirm)
  - Context actions via `a` (IDEA "Actions" mnemonic) or right-click menu (via `Shift-A`)

---

## P4: Branch & Remote Operations

### Task 4.1: Branch operations

- **Create:** `internal/gitops/branch.go`
  - `Branches() ([]Branch, error)` local + remote
  - `Checkout(name string) error`
  - `CreateBranch(name, base string) error`
  - `DeleteBranch(name string, force bool) error`
  - `SetUpstream(local, remote string) error`

### Task 4.2: Merge & rebase

- **Create:** `internal/gitops/merge_rebase.go`
  - `Merge(branch string) (bool, error)` returns hasConflicts
  - `Rebase(branch string) error`
  - `RebaseContinue()`, `RebaseAbort()`, `RebaseSkip()`
  - `Pull(remote, branch string) error` (fetch + merge/rebase per config)
  - Detect rebase-in-progress via `.git/rebase-merge` or git2go state
  - If conflicts after merge/rebase: switch filelist to "Conflicted" section highlight, show "Resolve All" hint in cmdbar

### Task 4.3: Remote operations

- **Create:** `internal/gitops/remote.go`
  - `Fetch(remote string) error`
  - `Push(remote, refspec string) error`
  - `Remotes() []Remote`

---

## P5: IDEA Extras

### Task 5.1: Shelve (IDEA-style, distinct from stash)

- **Create:** `internal/gitops/shelve.go`
  - Shelves stored as patch files in `.cogit/shelves/` (outside git objects, user-visible)
  - `Shelve(name string, paths []string) error` — create patch from working changes, then reset those paths
  - `Unshelve(name string) error` — apply patch
  - `DeleteShelve(name string) error`
  - `ListShelves() []ShelveInfo`
  - UI: sidebar "Shelves" tree, `Enter` to unshelve, `D` to delete, `N` to create new shelve from current changes
  - This mirrors IDEA's "Shelve Changes" exactly.

### Task 5.2: Ignore manager

- **Create:** `internal/gitops/ignore.go`
  - `IgnorePatterns(dir string) ([]string, error)` read `.gitignore`
  - `AddIgnore(dir, pattern string) error`
  - `RemoveIgnore(dir, pattern string) error`
  - UI: select untracked/ignored file → `i` opens ignore dialog with suggested pattern (e.g., `*.log`, `/build/`), choose scope (root or subdir)

### Task 5.3: Stash

- **Create:** `internal/gitops/stash.go`
  - `StashSave(msg string, includeUntracked bool) error`
  - `StashPop(index int) error`
  - `StashApply(index int) error`
  - `StashDrop(index int) error`
  - `StashList() []StashEntry`
  - UI: sidebar under Stashes tree

### Task 5.4: Cherry-pick & actions

- **Create:** `internal/gitops/commit.go` extension
  - `CherryPick(oid string) error`, `CherryPickAbort()`, `CherryPickContinue()`
  - `Commit(message string) error`, `CommitAmend(message string) error`
  - Command palette `:cp <ref>` for cherry-pick

---

## P6: Polish

### Task 6.1: Command palette

- **Create:** `internal/panels/cmdpalette.go` (or inline in app)
  - `:` opens bottom input (like vim cmdline)
  - Commands: `:q`, `:q!`, `:wqa`, `:cp`, `:merge`, `:rebase`, `:fetch`, `:push`, `:pull`, `:commit`, `:stash`, `:shelve`, `:branch`
  - Tab completion for commands and refs

### Task 6.2: Help system

- **Create:** `internal/app/help.go`
  - `?` opens full-screen help with current-mode keybindings
  - Context-aware: if in filelist, show filelist keys; if in diff, show diff keys

### Task 6.3: Theming & config

- **Create:** default config template generation
  - `cogit init` to write default config
  - Themes: dark (default), light, high-contrast

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

## libgit2 / git2go Notes

- Use `github.com/libgit2/git2go/v34`
- Requires libgit2 shared library at runtime.
- Build tags may be needed on some platforms.
- For line-level staging: libgit2 supports patch application; we can use `git.Apply` with hunks for hunk-level stage/unstage.
- For diff: `git.Diff` APIs provide unified diff text; parse into hunks ourselves.

---

## Risks & Mitigations

1. **CGO/libgit2 distribution:** Users must install libgit2. Mitigation: document install steps; future: static link or switch to go-git for portable builds.
2. **Rebase/merge conflict UI:** Complex state machine. Mitigation: detect state via filesystem markers + git2go state; show clear "REBASE" indicator in cmdbar.
3. **Performance on large repos:** Status/diff may be slow. Mitigation: limit file list height + virtual scrolling; async status updates via `tea.Cmd`.
4. **Windows compatibility:** libgit2 on Windows is fiddly. Mitigation: document MSYS2/vcpkg steps; initial target macOS/Linux.

---

## Next Step

Confirm plan → initialize repo on GitHub → delegate to opencode task-by-task.
