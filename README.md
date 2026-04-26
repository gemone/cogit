# cogit

A Rust TUI for Git workflows, inspired by [lazygit](https://github.com/jesseduffield/lazygit) and the IDEA Git tool window.

## Current focus

- branch, log, stash, shelve, remote, diff, and commit flows
- vim-style navigation and fast keyboard-driven operations

## Run

```bash
cargo run -- <repo-path>
```

## Config

cogit reads `config.toml` from the standard config directory:

- macOS: `~/Library/Application Support/one.gemo.cogit/config.toml`
- Linux: `~/.config/cogit/config.toml` or `$XDG_CONFIG_HOME/cogit/config.toml`
- the file is created on demand when switching keymap presets or saving a global layout preset

Example:

```toml
[keymap]
preset = "vim" # or "helix"

[layout]
vertical = [82, 18]
columns = [28, 44, 28]
left_rows = [48, 28, 24]
center_rows = [68, 32]
right_rows = [52, 48]
```

Tiled layout notes:

- The main screen now keeps every panel visible in a shared lazygit-style tiled layout.
- `Tab` / `Shift+Tab` moves focus between panes.
- `Ctrl+Arrow` resizes the active pane group.
- `Ctrl+s` saves the current layout into the repo-local `.git/config` as `cogit.layout`.
- `Ctrl+g` saves the current layout as the global default in `config.toml`.
- `Ctrl+r` clears the repo-local override and reloads the saved default layout.

In-app commands:

- `:keymap vim`
- `:keymap helix`
- `:keymap` to show the current preset
- `?` to open contextual which-key/help

## Notes

- Planning documents are kept outside the repository under `.hermes/` and are not tracked in git.
