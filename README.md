# cogit

A Rust TUI for Git workflows, inspired by Lazygit and the IDEA Git tool window.

## Current focus

- branch, log, stash, shelve, remote, diff, and commit flows
- vim-style navigation and fast keyboard-driven operations

## Run

```bash
cargo run -- <repo-path>
```

## Config

cogit reads `config.toml` from the standard config directory:

- macOS: `~/Library/Application Support/io.gemone.cogit/config.toml`
- Linux: `~/.config/cogit/config.toml` or `$XDG_CONFIG_HOME/cogit/config.toml`
- the file is created on demand when switching keymap presets

Example:

```toml
[keymap]
preset = "vim" # or "helix"
```

In-app commands:

- `:keymap vim`
- `:keymap helix`
- `:keymap` to show the current preset
- `?` to open contextual which-key/help

## Notes

- Planning documents are kept outside the repository under `.hermes/` and are not tracked in git.
