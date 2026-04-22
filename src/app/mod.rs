pub mod cmdline;
pub mod help;
pub mod notification;
pub mod styles;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::path::Path;

use crate::gitops::Repository;
use crate::panels::{
    Action, Panel, branch_panel::BranchPanel, filelist_panel::FileListPanel, log_panel::LogPanel,
    stash_panel::StashPanel,
};
use crate::vimkeys::Mode;

use self::cmdline::CmdLine;
use self::help::HelpOverlay;
use self::notification::NotificationManager;
use self::styles::Styles;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Main,
    Branches,
    Log,
    Stash,
}

pub struct App {
    repo_path: std::path::PathBuf,
    repo: Repository,
    view: View,
    mode: Mode,
    styles: Styles,
    should_quit: bool,
    // Panels
    filelist: FileListPanel,
    branch_panel: BranchPanel,
    log_panel: LogPanel,
    stash_panel: StashPanel,
    // UI components
    cmdline: CmdLine,
    notifications: NotificationManager,
    diff_popup: Option<(String, String, u16)>, // (path, content, scroll)
    commit_dialog: Option<String>,             // commit message input
    branch_dialog: Option<String>,              // branch name input
    reset_dialog: Option<(String, String, bool)>, // (mode, path, selecting_mode)
    help_overlay: HelpOverlay,
}

impl App {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        let styles = Styles::default();
        let filelist = FileListPanel::new(repo_path, &styles);
        let branch_panel = BranchPanel::new(repo_path, &styles);
        let log_panel = LogPanel::new(repo_path, &styles);
        let stash_panel = StashPanel::new(repo_path, &styles);
        let cmdline = CmdLine::new(&styles);
        let notifications = NotificationManager::new();
        let help_overlay = HelpOverlay::new(&styles);

        Ok(Self {
            repo_path: repo_path.to_path_buf(),
            repo,
            view: View::Main,
            mode: Mode::Normal,
            styles,
            should_quit: false,
            filelist,
            branch_panel,
            log_panel,
            stash_panel,
            cmdline,
            notifications,
            diff_popup: None,
            commit_dialog: None,
            branch_dialog: None,
            reset_dialog: None,
            help_overlay,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.draw_frame(f))?;
            if event::poll(std::time::Duration::from_millis(100))?
                && let Event::Key(key) = event::read()? {
                    self.handle_event(key);
                }
            self.notifications.cleanup();
        }
        Ok(())
    }

    fn handle_event(&mut self, key: KeyEvent) {
        // Diff popup takes priority
        if self.diff_popup.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.diff_popup = None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_add(1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_sub(1);
                    }
                }
                KeyCode::Char('G') => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = u16::MAX;
                    }
                }
                KeyCode::Char('g') => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = 0;
                    }
                }
                KeyCode::PageDown | KeyCode::Char('J') => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_add(15);
                    }
                }
                KeyCode::PageUp | KeyCode::Char('K') => {
                    if let Some((_, _, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_sub(15);
                    }
                }
                _ => {}
            }
            return;
        }

        // Commit dialog takes priority
        if let Some(ref mut msg) = self.commit_dialog {
            match key.code {
                KeyCode::Esc => {
                    self.commit_dialog = None;
                }
                KeyCode::Enter => {
                    let msg = std::mem::take(msg);
                    self.commit_dialog = None;
                    if !msg.is_empty() {
                        self.dispatch(Action::Commit(msg));
                    } else {
                        self.notifications.notify_error("Empty commit message");
                    }
                }
                KeyCode::Backspace => {
                    msg.pop();
                }
                KeyCode::Char(c) => {
                    msg.push(c);
                }
                _ => {}
            }
            return;
        }

        // Branch dialog takes priority
        if let Some(ref mut name) = self.branch_dialog {
            match key.code {
                KeyCode::Esc => {
                    self.branch_dialog = None;
                }
                KeyCode::Enter => {
                    let name = std::mem::take(name);
                    self.branch_dialog = None;
                    if !name.is_empty() {
                        self.dispatch(Action::CreateBranch(name));
                    }
                }
                KeyCode::Backspace => {
                    name.pop();
                }
                KeyCode::Char(c) => {
                    name.push(c);
                }
                _ => {}
            }
            return;
        }

        // Reset dialog takes priority (for Ctrl+u in filelist)
        if let Some(ref mut reset_state) = self.reset_dialog {
            match key.code {
                KeyCode::Esc => {
                    self.reset_dialog = None;
                }
                KeyCode::Enter => {
                    let (mode, path, _) = std::mem::take(reset_state);
                    self.reset_dialog = None;
                    if !mode.is_empty() {
                        self.dispatch(Action::Reset(mode, path));
                    }
                }
                KeyCode::Char('1') => {
                    // soft
                    reset_state.0 = "soft".to_string();
                    reset_state.2 = false;
                }
                KeyCode::Char('2') => {
                    // hard
                    reset_state.0 = "hard".to_string();
                    reset_state.2 = false;
                }
                KeyCode::Char('3') => {
                    // mixed (default)
                    reset_state.0 = "mixed".to_string();
                    reset_state.2 = false;
                }
                KeyCode::Backspace => {
                    if reset_state.2 {
                        self.reset_dialog = None;
                    } else if reset_state.1.is_empty() {
                        reset_state.2 = true;
                    } else {
                        reset_state.1.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if reset_state.2 {
                        reset_state.2 = false;
                        reset_state.1.clear();
                    }
                    reset_state.1.push(c);
                }
                _ => {}
            }
            return;
        }

        // Help overlay takes priority
        if self.help_overlay.is_visible() {
            self.help_overlay.handle_key(key);
            return;
        }

        // Command mode takes priority
        if self.cmdline.is_visible() {
            self.handle_command_key(key);
            return;
        }

        if key.code == KeyCode::Char('?') {
            self.dispatch(Action::Help);
            return;
        }

        if self.mode == Mode::Command {
            self.mode = Mode::Normal;
        }

        match self.view {
            View::Main => self.handle_main_key(key),
            View::Branches => {
                if let Some(action) = self.branch_panel.handle_key(key) {
                    self.dispatch(action);
                }
            }
            View::Log => {
                if let Some(action) = self.log_panel.handle_key(key) {
                    self.dispatch(action);
                }
            }
            View::Stash => {
                if let Some(action) = self.stash_panel.handle_key(key) {
                    self.dispatch(action);
                }
            }
        }
    }

    fn handle_main_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.cmdline.open();
            }
            KeyCode::Char('1') => {
                self.switch_view(View::Branches);
            }
            KeyCode::Char('2') => {
                self.switch_view(View::Log);
            }
            KeyCode::Char('4') => {
                self.switch_view(View::Stash);
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.dispatch(Action::Help);
            }
            KeyCode::Char('s') => {
                self.dispatch(Action::Stage);
            }
            KeyCode::Char('S') => {
                self.dispatch(Action::StageAll);
            }
            KeyCode::Char('u') => {
                self.dispatch(Action::Unstage);
            }
            KeyCode::Char('U') => {
                self.dispatch(Action::UnstageAll);
            }
            KeyCode::Char('c') => {
                self.dispatch(Action::CommitDialog);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.filelist.handle_key(key);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.filelist.handle_key(key);
            }
            _ => {
                if let Some(action) = self.filelist.handle_key(key) {
                    self.dispatch(action);
                }
            }
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.cmdline.close();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let cmd = self.cmdline.submit();
                self.mode = Mode::Normal;
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                self.cmdline.backspace();
            }
            KeyCode::Char(c) => {
                self.cmdline.input_char(c);
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        // Parse commit -m "msg"
        if let Some(msg) = cmd.strip_prefix(":commit -m ") {
            let msg = msg.trim_matches('"').trim();
            self.dispatch(Action::Commit(msg.to_string()));
            return;
        }
        // Parse checkout X
        if let Some(branch) = cmd.strip_prefix(":checkout ") {
            let branch = branch.trim();
            self.dispatch(Action::CheckoutBranch(branch.to_string()));
            return;
        }
        // Parse tag <name> (create tag)
        if let Some(name) = cmd.strip_prefix(":tag ") {
            let name = name.trim();
            if !name.is_empty() {
                self.dispatch(Action::CreateTag(name.to_string()));
            }
            return;
        }
        // Parse reset [path] [soft|hard|mixed]
        if let Some(args) = cmd.strip_prefix(":reset ") {
            let parts: Vec<&str> = args.split_whitespace().collect();
            let (mode, path) = match parts.as_slice() {
                [] => ("mixed".to_string(), "".to_string()),
                [m] if *m == "soft" || *m == "hard" || *m == "mixed" => (m.to_string(), "".to_string()),
                [p] => ("mixed".to_string(), p.to_string()),
                [m, p] if *m == "soft" || *m == "hard" || *m == "mixed" => (m.to_string(), p.to_string()),
                [p, m] if *m == "soft" || *m == "hard" || *m == "mixed" => (m.to_string(), p.to_string()),
                _ => ("mixed".to_string(), args.trim().to_string()),
            };
            self.dispatch(Action::Reset(mode, path));
            return;
        }

        let action = match cmd {
            ":w" | ":stage" => Action::Stage,
            ":wq" => Action::CommitDialog,
            ":q" | ":q!" => Action::Quit,
            ":stageall" => Action::StageAll,
            ":unstage" => Action::Unstage,
            ":unstageall" => Action::UnstageAll,
            ":commit" | ":c" => Action::CommitDialog,
            ":reset" => Action::Reset("mixed".to_string(), "".to_string()),
            ":reset-soft" => Action::Reset("soft".to_string(), "".to_string()),
            ":reset-hard" => Action::Reset("hard".to_string(), "".to_string()),
            ":reset-mixed" => Action::Reset("mixed".to_string(), "".to_string()),
            ":wip" => Action::WipCommit,
            ":discard" => Action::Discard,
            ":stash" => Action::ShowStashPanel,
            ":stashpop" => Action::StashPop(0),
            ":push" => Action::PushCurrent,
            ":fetch" => Action::FetchAll,
            ":branch" | ":branches" => Action::ShowBranchPanel,
            ":log" => Action::ShowLogPanel,
            ":help" => Action::Help,
            ":amend" => Action::AmendCommit,
            ":tag" | ":tags" => Action::ShowTags,
            "" => return,
            _ => {
                self.notifications
                    .notify_error(&format!("Unknown command: {}", cmd));
                return;
            }
        };
        self.dispatch(action);
    }

    fn switch_view(&mut self, view: View) {
        // Blur current panels
        self.filelist.blur();
        self.branch_panel.blur();
        self.log_panel.blur();
        self.stash_panel.blur();

        self.view = view.clone();
        match &self.view {
            View::Main => {
                self.filelist.focus();
            }
            View::Branches => {
                self.branch_panel.focus();
            }
            View::Log => {
                self.log_panel.focus();
            }
            View::Stash => {
                self.stash_panel.focus();
            }
        }
    }

    fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::BackToMain => {
                self.switch_view(View::Main);
            }
            Action::ShowBranchPanel => {
                self.switch_view(View::Branches);
            }
            Action::ShowLogPanel => {
                self.switch_view(View::Log);
            }
            Action::ShowStashPanel => {
                self.switch_view(View::Stash);
            }
            Action::Stage => {
                self.dispatch_stage();
            }
            Action::StageAll => {
                if let Err(e) = self.repo.stage_all() {
                    self.notifications
                        .notify_error(&format!("Stage all failed: {}", e));
                } else {
                    self.notifications.notify("All files staged");
                    self.refresh_all();
                }
            }
            Action::Unstage => {
                self.dispatch_unstage();
            }
            Action::UnstageAll => {
                if let Err(e) = self.repo.unstage_all() {
                    self.notifications
                        .notify_error(&format!("Unstage all failed: {}", e));
                } else {
                    self.notifications.notify("All files unstaged");
                    self.refresh_all();
                }
            }
            Action::Discard => {
                self.dispatch_discard();
            }
            Action::CommitDialog => {
                let staged = self.filelist.files.iter().filter(|f| f.staged).count();
                if staged == 0 {
                    self.notifications
                        .notify_error("Nothing staged. Stage files first (s/Space)");
                } else {
                    self.commit_dialog = Some(String::new());
                }
            }
            Action::Commit(msg) => match self.repo.commit(&msg) {
                Ok(_) => {
                    self.notifications.notify(&format!("Committed: {}", msg));
                    self.refresh_all();
                }
                Err(e) => self
                    .notifications
                    .notify_error(&format!("Commit failed: {}", e)),
            },
            Action::WipCommit => match self.repo.wip_commit() {
                Ok(_) => {
                    self.notifications.notify("WIP commit created");
                    self.refresh_all();
                }
                Err(e) => self
                    .notifications
                    .notify_error(&format!("WIP commit failed: {}", e)),
            },
            Action::Reset(mode, path) => match self.repo.reset(&mode, &path) {
                Ok(_) => {
                    let msg = if path.is_empty() {
                        format!("Reset {} (whole repo)", mode)
                    } else {
                        format!("Reset {} {}", mode, path)
                    };
                    self.notifications.notify(&msg);
                    self.refresh_all();
                }
                Err(e) => self
                    .notifications
                    .notify_error(&format!("Reset failed: {}", e)),
            },
            Action::ResetDialog(mode) => {
                // Open reset dialog with pre-selected mode
                self.reset_dialog = Some((mode, String::new(), true));
            }
            Action::AmendCommit => match self.repo.amend_commit(None) {
                Ok(_) => {
                    self.notifications.notify("Commit amended");
                    self.refresh_all();
                }
                Err(e) => self
                    .notifications
                    .notify_error(&format!("Amend failed: {}", e)),
            },
            Action::CheckoutBranch(name) => match self.repo.checkout(&name) {
                Ok(_) => {
                    self.notifications.notify(&format!("Switched to {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Checkout failed: {}", e));
                }
            },
            Action::PushCurrent => match self.repo.push_current() {
                Ok(_) => {
                    self.notifications.notify("Pushed successfully");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Push failed: {}", e));
                }
            },
            Action::FetchAll => match self.repo.fetch_all() {
                Ok(_) => {
                    self.notifications.notify("Fetched all remotes");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Fetch failed: {}", e));
                }
            },
            Action::PullCurrent => match self.repo.pull_current() {
                Ok(_) => {
                    self.notifications.notify("Pulled successfully");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Pull failed: {}", e));
                }
            },
            Action::CreateBranch(name) => match self.repo.create_branch(&name) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Created branch: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Create branch failed: {}", e));
                }
            },
            Action::CreateBranchDialog => {
                self.branch_dialog = Some(String::new());
            }
            Action::DeleteBranch(name) => match self.repo.delete_branch(&name) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Deleted branch: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Delete branch failed: {}", e));
                }
            },
            Action::MergeBranch(name) => match self.repo.merge(&name) {
                Ok(_) => {
                    self.notifications.notify(&format!("Merged: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Merge failed: {}", e));
                }
            },
            Action::RebaseBranch(name) => match self.repo.rebase(&name) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Rebased onto: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Rebase failed: {}", e));
                }
            },
            Action::CherryPick(hash) => match self.repo.cherry_pick(&hash) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Cherry-picked: {}", &hash[..7.min(hash.len())]));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Cherry-pick failed: {}", e));
                }
            },
            Action::CopyHash(hash) => {
                // Copy to clipboard using arboard
                use arboard::Clipboard;
                if let Ok(mut clipboard) = Clipboard::new() {
                    if clipboard.set_text(&hash).is_ok() {
                        self.notifications
                            .notify(&format!("Copied: {}", &hash[..7.min(hash.len())]));
                    } else {
                        self.notifications.notify_error("Failed to copy to clipboard");
                    }
                } else {
                    self.notifications.notify_error("Failed to access clipboard");
                }
            }
            Action::SearchLog(_query) => {
                // Handled by log panel directly
            }
            Action::ShowTags => {
                match self.repo.tag_list() {
                    Ok(tags) => {
                        if tags.is_empty() {
                            self.notifications.notify("No tags found");
                        } else {
                            let tag_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();
                            self.notifications.notify(&format!("Tags: {}", tag_names.join(", ")));
                        }
                    }
                    Err(e) => {
                        self.notifications
                            .notify_error(&format!("Failed to list tags: {}", e));
                    }
                }
            }
            Action::CreateTag(name) => match self.repo.tag_create(&name, "", None) {
                Ok(_) => {
                    self.notifications.notify(&format!("Created tag: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Create tag failed: {}", e));
                }
            },
            Action::DeleteTag(name) => match self.repo.tag_delete(&name) {
                Ok(_) => {
                    self.notifications.notify(&format!("Deleted tag: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Delete tag failed: {}", e));
                }
            },
            Action::Stash => match self.repo.stash_create(None) {
                Ok(_) => {
                    self.notifications.notify("Stash created");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Stash failed: {}", e));
                }
            },
            Action::StashPop(index) => match self.repo.stash_pop(index) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Stash@{} popped", index));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Stash pop failed: {}", e));
                }
            },
            Action::StashApply(index) => match self.repo.stash_apply(index) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Stash@{} applied", index));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Stash apply failed: {}", e));
                }
            },
            Action::StashDrop(index) => match self.repo.stash_drop(index) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Stash@{} dropped", index));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Stash drop failed: {}", e));
                }
            },
            Action::ShelveApply(name) => match self.repo.shelve_apply(&name) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Shelve '{}' applied", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve apply failed: {}", e));
                }
            },
            Action::ShelveDrop(name) => match self.repo.shelve_drop(&name) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Shelve '{}' deleted", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve drop failed: {}", e));
                }
            },
            Action::ShelveCreate => match self.repo.shelve_create("default") {
                Ok(_) => {
                    self.notifications.notify("Shelve created");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve create failed: {}", e));
                }
            },
            Action::Help => {
                self.help_overlay.open();
            }
            Action::ShowDiff(path) => {
                let content = self
                    .repo
                    .file_diff(&path)
                    .unwrap_or_else(|e| format!("Error: {}", e));
                self.diff_popup = Some((path, content, 0));
            }
        }
    }

    fn dispatch_stage(&mut self) {
        // Stage the currently selected file
        let i = self.filelist.state.selected().unwrap_or(0);
        if let Some(file) = self.filelist.files.get(i) {
            if let Err(e) = self.repo.stage(&file.path) {
                self.notifications
                    .notify_error(&format!("Stage failed: {}", e));
            } else {
                self.notifications.notify(&format!("Staged: {}", file.path));
                self.refresh_all();
            }
        }
    }

    fn dispatch_unstage(&mut self) {
        let i = self.filelist.state.selected().unwrap_or(0);
        if let Some(file) = self.filelist.files.get(i) {
            if let Err(e) = self.repo.unstage(&file.path) {
                self.notifications
                    .notify_error(&format!("Unstage failed: {}", e));
            } else {
                self.notifications
                    .notify(&format!("Unstaged: {}", file.path));
                self.refresh_all();
            }
        }
    }

    fn dispatch_discard(&mut self) {
        let i = self.filelist.state.selected().unwrap_or(0);
        if let Some(file) = self.filelist.files.get(i) {
            if let Err(e) = self.repo.discard(&file.path) {
                self.notifications
                    .notify_error(&format!("Discard failed: {}", e));
            } else {
                self.notifications
                    .notify(&format!("Discarded: {}", file.path));
                self.refresh_all();
            }
        }
    }

    fn refresh_all(&mut self) {
        self.filelist.refresh();
        self.branch_panel.refresh();
        self.log_panel.refresh();
        self.stash_panel.refresh();
    }

    fn draw_frame(&mut self, f: &mut Frame) {
        let size = f.area();

        // Background
        let bg = Block::default().style(Style::default().bg(ratatui::style::Color::Reset));
        f.render_widget(bg, size);

        // Reserve bottom row for command line when visible
        let view_area = if self.cmdline.is_visible() {
            Rect {
                height: size.height.saturating_sub(1),
                ..size
            }
        } else {
            size
        };

        match self.view {
            View::Main => self.draw_main(f, view_area),
            View::Branches => self.branch_panel.render(f, view_area),
            View::Log => self.log_panel.render(f, view_area),
            View::Stash => self.stash_panel.render(f, view_area),
        }

        // Command line at bottom
        if self.cmdline.is_visible() {
            let cmd_area = Rect {
                x: 0,
                y: size.height.saturating_sub(1),
                width: size.width,
                height: 1,
            };
            self.cmdline.render(f, cmd_area);
        }

        // Diff popup overlay
        if let Some((ref path, ref content, scroll)) = self.diff_popup {
            self.draw_diff_popup(f, size, path, content, scroll);
        }

        // Commit dialog overlay
        if let Some(ref msg) = self.commit_dialog {
            self.draw_commit_dialog(f, size, msg);
        }

        // Branch dialog overlay
        if let Some(ref name) = self.branch_dialog {
            self.draw_branch_dialog(f, size, name);
        }

        // Reset dialog overlay
        if let Some(ref reset_state) = self.reset_dialog {
            self.draw_reset_dialog(f, size, reset_state);
        }

        // Notifications on top of everything
        self.notifications.render(f, size);

        // Help overlay modal on top of everything
        self.help_overlay.render(f, size);
    }

    fn draw_main(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status bar
                Constraint::Min(5),    // File list
                Constraint::Length(1), // Sidebar help
            ])
            .split(area);

        // Status bar
        let branch = self.repo.current_branch().unwrap_or_default();
        let status_bar = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", branch),
                self.styles.addition.add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cogit ", self.styles.text_secondary),
        ]))
        .style(Style::default().bg(ratatui::style::Color::DarkGray));
        f.render_widget(status_bar, chunks[0]);

        // File list
        self.filelist.focus();
        self.filelist.render(f, chunks[1]);
        let help = Paragraph::new(
            " 1:branches 2:log 4:stash ?:help :commands s:stage S:stage-all c:commit q:quit",
        )
        .style(self.styles.text_secondary);
        f.render_widget(help, chunks[2]);
    }

    fn draw_diff_popup(&self, f: &mut Frame, area: Rect, path: &str, content: &str, scroll: u16) {
        // Centered popup: 80% width, 80% height
        let popup_w = (area.width * 4 / 5).max(40);
        let popup_h = (area.height * 4 / 5).max(10);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        // Clear background
        let clear = Block::default().style(
            Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(3),
        };

        // Color diff lines
        let lines: Vec<Line> = content
            .lines()
            .map(|line| {
                let style = if line.starts_with('+') && !line.starts_with("+++") {
                    Style::default().fg(ratatui::style::Color::Green)
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Style::default().fg(ratatui::style::Color::Red)
                } else if line.starts_with("@@") {
                    Style::default().fg(ratatui::style::Color::Cyan)
                } else {
                    Style::default().fg(ratatui::style::Color::White)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let title = format!(" Diff: {} (j/k:scroll G/g:jump PgUp/PgDn) ", path);
        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(ratatui::style::Color::Yellow)),
            )
            .scroll((scroll, 0));

        f.render_widget(paragraph, inner);
    }

    fn draw_commit_dialog(&self, f: &mut Frame, area: Rect, msg: &str) {
        let popup_w = (area.width * 3 / 5).max(40);
        let popup_h = 9;
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(
            Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let staged_count = self.filelist.files.iter().filter(|f| f.staged).count();

        let input_display = if msg.is_empty() {
            "type commit message...".to_string()
        } else {
            msg.to_string()
        };

        let lines = vec![
            Line::from(Span::styled(
                format!("  {} file(s) staged", staged_count),
                Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("> ", Style::default().fg(ratatui::style::Color::Yellow)),
                Span::styled(
                    input_display,
                    if msg.is_empty() {
                        Style::default().fg(ratatui::style::Color::DarkGray)
                    } else {
                        Style::default()
                            .fg(ratatui::style::Color::White)
                            .add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("█", Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Enter: commit  |  Esc: cancel",
                Style::default().fg(ratatui::style::Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Commit ")
                .border_style(Style::default().fg(ratatui::style::Color::Green)),
        );
        f.render_widget(paragraph, popup_area);
    }

    fn draw_branch_dialog(&self, f: &mut Frame, area: Rect, name: &str) {
        let popup_w = (area.width * 3 / 5).max(40);
        let popup_h = 7;
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(
            Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let input_display = if name.is_empty() {
            "type branch name...".to_string()
        } else {
            name.to_string()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("> ", Style::default().fg(ratatui::style::Color::Yellow)),
                Span::styled(
                    input_display,
                    if name.is_empty() {
                        Style::default().fg(ratatui::style::Color::DarkGray)
                    } else {
                        Style::default()
                            .fg(ratatui::style::Color::White)
                            .add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("█", Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Enter: create  |  Esc: cancel",
                Style::default().fg(ratatui::style::Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" New Branch ")
                .border_style(Style::default().fg(ratatui::style::Color::Cyan)),
        );
        f.render_widget(paragraph, popup_area);
    }

    fn draw_reset_dialog(&self, f: &mut Frame, area: Rect, reset_state: &(String, String, bool)) {
        let (mode, path, selecting_mode) = reset_state;
        let popup_w = (area.width * 3 / 5).max(40);
        let popup_h = 11;
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(
            Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let path_display = if path.is_empty() && !selecting_mode {
            "type path (or Enter for whole repo)".to_string()
        } else if path.is_empty() && *selecting_mode {
            "type path...".to_string()
        } else {
            path.clone()
        };

        let lines = vec![
            Line::from(Span::styled(
                " Reset ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Mode: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                if mode.is_empty() {
                    Span::styled("1:soft  2:hard  3:mixed", Style::default().fg(ratatui::style::Color::Cyan))
                } else {
                    Span::styled(mode.as_str(), Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD))
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("> Path: ", Style::default().fg(ratatui::style::Color::Yellow)),
                Span::styled(
                    &path_display,
                    if path.is_empty() {
                        Style::default().fg(ratatui::style::Color::DarkGray)
                    } else {
                        Style::default()
                            .fg(ratatui::style::Color::White)
                            .add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("█", Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  1/2/3: select mode  |  Enter: reset  |  Esc: cancel",
                Style::default().fg(ratatui::style::Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Reset ")
                .border_style(Style::default().fg(ratatui::style::Color::Red)),
        );
        f.render_widget(paragraph, popup_area);
    }
}
