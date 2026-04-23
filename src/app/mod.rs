pub mod cmdline;
pub mod help;
pub mod navigation;
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
use crate::gitops::shell::{MergePreview, MergeStrategy};
use crate::panels::{
    Action, Panel, branch_panel::BranchPanel, filelist_panel::FileListPanel, log_panel::LogPanel,
    remote_panel, shelve_panel::ShelvePanel, stash_panel::StashPanel,
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
    Remote,
    Shelve,
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
    remote_panel: remote_panel::RemotePanel,
    shelve_panel: ShelvePanel,
    // UI components
    cmdline: CmdLine,
    notifications: NotificationManager,
    diff_popup: Option<(String, String, u16)>, // (path, content, scroll)
    ref_diff_popup: Option<(String, String, u16)>, // (title "from..to", content, scroll)
    commit_dialog: Option<String>,             // commit message input
    branch_dialog: Option<String>,              // branch name input
    rename_dialog: Option<(String, String)>, // (old_name, new_name being typed)
    reset_dialog: Option<(String, String, bool)>, // (mode, path, selecting_mode)
    pending_checkout: Option<(String, bool)>, // (branch name, is_remote) waiting for stash confirmation
    gitignore_popup: Option<(String, u16)>, // (content, scroll)
    merge_dialog: Option<(String, MergePreview)>, // (branch, preview)
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
        let remote_panel = remote_panel::RemotePanel::new(repo_path, &styles);
        let shelve_panel = ShelvePanel::new(repo_path, &styles);
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
            remote_panel,
            shelve_panel,
            cmdline,
            notifications,
            diff_popup: None,
            ref_diff_popup: None,
            commit_dialog: None,
            branch_dialog: None,
            rename_dialog: None,
            reset_dialog: None,
            pending_checkout: None,
            gitignore_popup: None,
            merge_dialog: None,
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
        // Handle Esc/q to close any popup first
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            if self.ref_diff_popup.is_some() {
                self.ref_diff_popup = None;
                return;
            }
            if self.diff_popup.is_some() {
                self.diff_popup = None;
                return;
            }
            if self.gitignore_popup.is_some() {
                self.gitignore_popup = None;
                return;
            }
        }

        // Ref diff popup takes priority
        if self.ref_diff_popup.is_some()
            && let Some((_, _, scroll)) = self.ref_diff_popup.as_mut()
                && Self::handle_popup_scroll(Some(scroll), key.code) {
                    return;
                }

        // Diff popup takes priority
        if self.diff_popup.is_some()
            && let Some((_, _, scroll)) = self.diff_popup.as_mut()
                && Self::handle_popup_scroll(Some(scroll), key.code) {
                    return;
                }

        // Gitignore popup takes priority
        if self.gitignore_popup.is_some()
            && let Some((_, scroll)) = self.gitignore_popup.as_mut()
                && Self::handle_popup_scroll(Some(scroll), key.code) {
                    return;
                }

        // Ignore all other keys while a popup is active so they do not
        // fall through to the normal app dispatch.
        if self.ref_diff_popup.is_some()
            || self.diff_popup.is_some()
            || self.gitignore_popup.is_some()
        {
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

        // Rename dialog takes priority
        if let Some(ref mut rename_state) = self.rename_dialog {
            match key.code {
                KeyCode::Esc => {
                    self.rename_dialog = None;
                }
                KeyCode::Enter => {
                    let (old_name, new_name) = std::mem::take(rename_state);
                    self.rename_dialog = None;
                    self.dispatch(Action::RenameBranch(old_name, new_name));
                }
                KeyCode::Backspace => {
                    rename_state.1.pop();
                }
                KeyCode::Char(c) => {
                    rename_state.1.push(c);
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

        // Pending checkout confirmation (smart checkout dialog)
        if self.pending_checkout.is_some() {
            match key.code {
                KeyCode::Char('s') | KeyCode::Enter => {
                    if let Some((name, is_remote)) = self.pending_checkout.take() {
                        // Smart Checkout: shelve → checkout → unshelve
                        let shelve_name = format!("smart-checkout-{}-{}", name.replace('/', "-"), std::process::id());

                        match self.repo.shelve_create(&shelve_name, true) {
                            Ok(_) => {
                                match if is_remote {
                                    // For remote branches, try to checkout locally first
                                    self.repo.checkout_remote_branch(&name)
                                } else {
                                    self.repo.checkout(&name)
                                } {
                                    Ok(_) => {
                                        // Try to unshelve immediately
                                        match self.repo.shelve_apply(0, true) {
                                            Ok(_) => {
                                                self.notifications.notify(&format!("Smart checkout: switched to {} and restored changes", name));
                                            }
                                            Err(e) => {
                                                self.notifications.notify(&format!("Switched to {}, shelved changes kept (unshelve failed: {})", name, e));
                                            }
                                        }
                                        self.refresh_all();
                                    }
                                    Err(e) => {
                                        self.notifications.notify_error(&format!("Checkout failed: {}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                self.notifications.notify_error(&format!("Shelve failed: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char('f') => {
                    if let Some((name, _)) = self.pending_checkout.take() {
                        // Force Checkout: discard local changes and checkout
                        match self.repo.checkout_force(&name) {
                            Ok(_) => {
                                self.notifications.notify(&format!("Force checkout: switched to {} (local changes discarded)", name));
                                self.refresh_all();
                            }
                            Err(e) => {
                                self.notifications.notify_error(&format!("Force checkout failed: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.pending_checkout = None;
                    self.notifications.notify("Checkout cancelled");
                }
                _ => {}
            }
            return;
        }

        // Merge dialog takes priority
        if self.merge_dialog.is_some() {
            match key.code {
                KeyCode::Char('f') => {
                    // Fast-forward merge - consume the dialog and do the merge directly
                    if let Some((branch, _)) = self.merge_dialog.take() {
                        match self.repo.smart_merge(&branch, MergeStrategy::FastForward) {
                            Ok(output) => {
                                self.notifications.notify(&format!("Merged: {} {}", branch, output));
                                self.refresh_all();
                            }
                            Err(e) => {
                                self.notifications
                                    .notify_error(&format!("Merge failed: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char('n') => {
                    if let Some((branch, _)) = self.merge_dialog.take() {
                        match self.repo.smart_merge(&branch, MergeStrategy::NoFastForward) {
                            Ok(output) => {
                                self.notifications.notify(&format!("Merged (no-ff): {} {}", branch, output));
                                self.refresh_all();
                            }
                            Err(e) => {
                                self.notifications
                                    .notify_error(&format!("Merge failed: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char('s') => {
                    if let Some((branch, _)) = self.merge_dialog.take() {
                        match self.repo.smart_merge(&branch, MergeStrategy::Squash) {
                            Ok(output) => {
                                self.notifications.notify(&format!("Squash merged: {} {}", branch, output));
                                self.refresh_all();
                            }
                            Err(e) => {
                                self.notifications
                                    .notify_error(&format!("Squash merge failed: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.merge_dialog = None;
                    self.notifications.notify("Merge cancelled");
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
            View::Remote => {
                if let Some(action) = self.remote_panel.handle_key(key) {
                    self.dispatch(action);
                }
            }
            View::Shelve => {
                if let Some(action) = self.shelve_panel.handle_key(key) {
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
            KeyCode::Char('R') => {
                self.switch_view(View::Remote);
            }
            KeyCode::Char('S') => {
                self.switch_view(View::Shelve);
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
            KeyCode::Char('A') => {
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
        // Parse diff <ref1> <ref2>
        if let Some(args) = cmd.strip_prefix(":diff ") {
            let args = args.trim();
            // Support both "ref1..ref2" and "ref1 ref2" formats
            let range = if args.contains("..") {
                args.to_string()
            } else {
                let parts: Vec<&str> = args.split_whitespace().collect();
                if parts.len() == 2 {
                    format!("{}..{}", parts[0], parts[1])
                } else {
                    self.notifications.notify_error("Usage: :diff <ref1> <ref2> or :diff <ref1>..<ref2>");
                    return;
                }
            };
            self.dispatch(Action::ShowRefDiff(range));
            return;
        }
        // Parse rename-branch <old> <new>
        if let Some(args) = cmd.strip_prefix(":rename-branch ") {
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.len() == 2 {
                self.dispatch(Action::RenameBranch(parts[0].to_string(), parts[1].to_string()));
            } else {
                self.notifications.notify_error("Usage: :rename-branch <old_name> <new_name>");
            }
            return;
        }
        // Parse worktree add <path> <branch>
        if let Some(args) = cmd.strip_prefix(":worktree add ") {
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[0].to_string();
                let branch = if parts.len() > 1 { parts[1] } else { "HEAD" };
                self.dispatch(Action::CreateWorktree(path, branch.to_string()));
            } else {
                self.notifications.notify_error("Usage: :worktree add <path> <branch>");
            }
            return;
        }
        // Parse worktree remove <path>
        if let Some(path) = cmd.strip_prefix(":worktree remove ") {
            let path = path.trim();
            if !path.is_empty() {
                self.dispatch(Action::RemoveWorktree(path.to_string()));
            } else {
                self.notifications.notify_error("Usage: :worktree remove <path>");
            }
            return;
        }
        // Parse ignore (show .gitignore)
        if cmd == ":ignore" {
            self.dispatch(Action::ShowGitignore);
            return;
        }
        // Parse ignore-add <pattern>
        if let Some(pattern) = cmd.strip_prefix(":ignore-add ") {
            let pattern = pattern.trim();
            if !pattern.is_empty() {
                self.dispatch(Action::GitignoreAdd(pattern.to_string()));
            } else {
                self.notifications.notify_error("Usage: :ignore-add <pattern>");
            }
            return;
        }
        // Parse ignore-remove <pattern>
        if let Some(pattern) = cmd.strip_prefix(":ignore-remove ") {
            let pattern = pattern.trim();
            if !pattern.is_empty() {
                self.dispatch(Action::GitignoreRemove(pattern.to_string()));
            } else {
                self.notifications.notify_error("Usage: :ignore-remove <pattern>");
            }
            return;
        }
        // Parse ignore <pattern> (add mode)
        if let Some(pattern) = cmd.strip_prefix(":ignore ") {
            let pattern = pattern.trim();
            if !pattern.is_empty() {
                self.dispatch(Action::GitignoreAdd(pattern.to_string()));
            } else {
                // Show gitignore if no pattern
                self.dispatch(Action::ShowGitignore);
            }
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
            ":worktrees" => Action::ShowWorktrees,
            ":pull-rebase" | ":rebase-pull" => Action::PullRebase,
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
        self.remote_panel.blur();
        self.shelve_panel.blur();

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
            View::Remote => {
                self.remote_panel.focus();
            }
            View::Shelve => {
                self.shelve_panel.focus();
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
            Action::ShowRemotePanel => {
                self.switch_view(View::Remote);
            }
            Action::ShowShelvePanel => {
                self.switch_view(View::Shelve);
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
            Action::CheckoutBranch(name) => {
                // Check for uncommitted changes (staged, unstaged, and untracked)
                let has_changes = !self.filelist.files.is_empty();
                if has_changes {
                    self.pending_checkout = Some((name.clone(), false)); // false = local branch
                } else {
                    // No changes, proceed directly
                    match self.repo.checkout(&name) {
                        Ok(_) => {
                            self.notifications.notify(&format!("Switched to {}", name));
                            self.refresh_all();
                        }
                        Err(e) => {
                            self.notifications
                                .notify_error(&format!("Checkout failed: {}", e));
                        }
                    }
                }
            }
            Action::CheckoutRemoteBranch(name) => {
                let has_changes = !self.filelist.files.is_empty();
                if has_changes {
                    self.pending_checkout = Some((name.clone(), true)); // true = remote branch
                } else {
                    match self.repo.checkout_remote_branch(&name) {
                        Ok(msg) => {
                            self.notifications.notify(&msg);
                            self.refresh_all();
                        }
                        Err(e) => {
                            self.notifications
                                .notify_error(&format!("Checkout remote branch failed: {}", e));
                        }
                    }
                }
            }
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
            Action::MergeBranch(name) => {
                // If merge_dialog is already open, do the actual merge
                if self.merge_dialog.is_some() {
                    self.merge_dialog = None;
                    match self.repo.smart_merge(&name, MergeStrategy::FastForward) {
                        Ok(output) => {
                            self.notifications.notify(&format!("Merged: {} {}", name, output));
                            self.refresh_all();
                        }
                        Err(e) => {
                            self.notifications
                                .notify_error(&format!("Merge failed: {}", e));
                        }
                    }
                } else {
                    // Show preview dialog first
                    match self.repo.preview_merge(&name) {
                        Ok(preview) => {
                            if preview.has_conflicts {
                                self.notifications
                                    .notify_error(&format!("Merge '{}' would have conflicts ({} files changed, {} commits)",
                                        name, preview.files_changed.len(), preview.commits_count));
                            } else {
                                self.merge_dialog = Some((name, preview));
                            }
                        }
                        Err(e) => {
                            self.notifications
                                .notify_error(&format!("Failed to preview merge: {}", e));
                        }
                    }
                }
            }
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
            Action::RebaseContinue => match self.repo.rebase_continue() {
                Ok(_) => {
                    self.notifications.notify("Rebase continued");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Rebase continue failed: {}", e));
                }
            },
            Action::RebaseAbort => match self.repo.rebase_abort() {
                Ok(_) => {
                    self.notifications.notify("Rebase aborted");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Rebase abort failed: {}", e));
                }
            },
            Action::RebaseSkip => match self.repo.rebase_skip() {
                Ok(_) => {
                    self.notifications.notify("Rebase skipped");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Rebase skip failed: {}", e));
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
            Action::ShelveApplyOld(name) => match self.repo.shelve_apply_by_name(&name, false) {
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
            Action::ShelveDropOld(name) => match self.repo.shelve_drop_by_name(&name) {
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
            Action::ShelveCreateOld => match self.repo.shelve_create("default", false) {
                Ok(_) => {
                    self.notifications.notify("Shelve created");
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve create failed: {}", e));
                }
            },
            Action::ShelveCreate(name, include_staged) => match self.repo.shelve_create(&name, include_staged) {
                Ok(_) => {
                    self.notifications.notify(&format!("Shelve created: {}", name));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve create failed: {}", e));
                }
            },
            Action::ShelveApply(index, pop) => {
                let cmd_name = if pop { "popped" } else { "applied" };
                match self.repo.shelve_apply(index, pop) {
                    Ok(_) => {
                        self.notifications
                            .notify(&format!("Shelve@{} {}", index, cmd_name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications
                            .notify_error(&format!("Shelve {} failed: {}", cmd_name, e));
                    }
                }
            }
            Action::ShelveDrop(index) => match self.repo.shelve_drop(index) {
                Ok(_) => {
                    self.notifications
                        .notify(&format!("Shelve@{} dropped", index));
                    self.refresh_all();
                }
                Err(e) => {
                    self.notifications
                        .notify_error(&format!("Shelve drop failed: {}", e));
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
            Action::RenameBranchDialog(old_name) => {
                // Open rename dialog with (old_name, empty_input)
                self.rename_dialog = Some((old_name, String::new()));
            }
            Action::RenameBranch(old_name, new_name) => {
                if new_name.is_empty() {
                    self.notifications.notify_error("New branch name cannot be empty");
                } else {
                    match self.repo.rename_branch(&old_name, &new_name) {
                        Ok(_) => {
                            self.notifications.notify(&format!("Renamed branch: {} -> {}", old_name, new_name));
                            self.refresh_all();
                        }
                        Err(e) => {
                            self.notifications.notify_error(&format!("Rename failed: {}", e));
                        }
                    }
                }
            }
            Action::ShowRefDiff(range) => {
                // range format is "from..to"
                let parts: Vec<&str> = range.split("..").collect();
                if parts.len() == 2 {
                    let content = self.repo.diff_refs(parts[0], parts[1])
                        .unwrap_or_else(|e| format!("Error: {}", e));
                    self.ref_diff_popup = Some((range, content, 0));
                } else {
                    self.notifications.notify_error("Invalid ref range format. Use: from..to");
                }
            }
            Action::ShowWorktrees => {
                match self.repo.worktree_list() {
                    Ok(worktrees) => {
                        if worktrees.is_empty() {
                            self.notifications.notify("No worktrees found");
                        } else {
                            let info: Vec<String> = worktrees.iter().map(|w| {
                                let branch = w.branch.as_deref().unwrap_or("(detached)");
                                if w.is_main {
                                    format!("{} (main, branch: {})", w.path, branch)
                                } else {
                                    format!("{} (branch: {})", w.path, branch)
                                }
                            }).collect();
                            self.notifications.notify(&format!("Worktrees: {}", info.join("; ")));
                        }
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to list worktrees: {}", e));
                    }
                }
            }
            Action::CreateWorktree(path, branch) => {
                match self.repo.worktree_create(&path, &branch) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Created worktree: {} ({})", path, branch));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Create worktree failed: {}", e));
                    }
                }
            }
            Action::RemoveWorktree(path) => {
                match self.repo.worktree_remove(&path) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Removed worktree: {}", path));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Remove worktree failed: {}", e));
                    }
                }
            }
            Action::PullRebase => {
                match self.repo.pull_rebase_current() {
                    Ok(_) => {
                        self.notifications.notify("Pulled with rebase successfully");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Pull rebase failed: {}", e));
                    }
                }
            }
            Action::ShowGitignore => {
                match self.repo.gitignore_read() {
                    Ok(content) => {
                        if content.is_empty() {
                            self.notifications.notify(".gitignore is empty or does not exist");
                        } else {
                            self.gitignore_popup = Some((content, 0));
                        }
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to read .gitignore: {}", e));
                    }
                }
            }
            Action::GitignoreAdd(pattern) => {
                match self.repo.gitignore_add(&pattern) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Added to .gitignore: {}", pattern));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to add to .gitignore: {}", e));
                    }
                }
            }
            Action::GitignoreRemove(pattern) => {
                match self.repo.gitignore_remove(&pattern) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Removed from .gitignore: {}", pattern));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to remove from .gitignore: {}", e));
                    }
                }
            }
            Action::AddRemote(name, url) => {
                match self.repo.add_remote(&name, &url) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Added remote: {} -> {}", name, url));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to add remote: {}", e));
                    }
                }
            }
            Action::RemoveRemote(name) => {
                match self.repo.remove_remote(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Removed remote: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to remove remote: {}", e));
                    }
                }
            }
            Action::RenameRemote(old, new) => {
                match self.repo.rename_remote(&old, &new) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Renamed remote: {} -> {}", old, new));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to rename remote: {}", e));
                    }
                }
            }
            Action::FetchRemote(name) => {
                match self.repo.fetch_remote(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Fetched remote: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to fetch remote: {}", e));
                    }
                }
            }
            Action::ShowRemoteBranches(name) => {
                // Show branches for this remote as a notification
                match self.repo.branches() {
                    Ok(branches) => {
                        let remote_branches: Vec<String> = branches
                            .iter()
                            .filter(|b| b.name.starts_with(&format!("remotes/{}/", name)))
                            .map(|b| b.name.clone())
                            .collect();
                        if remote_branches.is_empty() {
                            self.notifications.notify(&format!("No branches found on remote: {}", name));
                        } else {
                            let branch_list = remote_branches.join(", ");
                            self.notifications.notify(&format!("{} branches: {}", name, branch_list));
                        }
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Failed to list branches: {}", e));
                    }
                }
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

    /// Handle popup scroll keys. Returns true if the key was handled.
    fn handle_popup_scroll(scroll: Option<&mut u16>, code: KeyCode) -> bool {
        let Some(scroll) = scroll else { return false; };
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                *scroll = scroll.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *scroll = scroll.saturating_sub(1);
                true
            }
            KeyCode::Char('G') => {
                *scroll = u16::MAX;
                true
            }
            KeyCode::Char('g') => {
                *scroll = 0;
                true
            }
            KeyCode::PageDown | KeyCode::Char('J') => {
                *scroll = scroll.saturating_add(15);
                true
            }
            KeyCode::PageUp | KeyCode::Char('K') => {
                *scroll = scroll.saturating_sub(15);
                true
            }
            _ => false,
        }
    }

    fn refresh_all(&mut self) {
        self.filelist.refresh();
        self.branch_panel.refresh();
        self.log_panel.refresh();
        self.stash_panel.refresh();
        self.remote_panel.refresh();
        self.shelve_panel.refresh();
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
            View::Remote => self.remote_panel.render(f, view_area),
            View::Shelve => self.shelve_panel.render(f, view_area),
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

        // Rename dialog overlay
        if let Some(ref rename_state) = self.rename_dialog {
            self.draw_rename_dialog(f, size, rename_state);
        }

        // Ref diff popup overlay
        if let Some((ref title, ref content, scroll)) = self.ref_diff_popup {
            self.draw_ref_diff_popup(f, size, title, content, &scroll);
        }

        // Reset dialog overlay
        if let Some(ref reset_state) = self.reset_dialog {
            self.draw_reset_dialog(f, size, reset_state);
        }

        // Merge dialog overlay
        if let Some(ref merge_state) = self.merge_dialog {
            self.draw_merge_dialog(f, size, merge_state);
        }

        // Smart checkout dialog overlay
        if let Some((ref branch, _)) = self.pending_checkout {
            self.draw_smart_checkout_dialog(f, size, branch);
        }

        // Gitignore popup overlay
        if let Some((ref content, scroll)) = self.gitignore_popup {
            self.draw_gitignore_popup(f, size, content, &scroll);
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
            " 1:branches 2:log 4:stash R:remote s:shelve S:stage-all c:commit q:quit ?:help :commands",
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

        self.draw_input_dialog(f, area, 9, " Commit ", ratatui::style::Color::Green, lines);
    }

    fn draw_branch_dialog(&self, f: &mut Frame, area: Rect, name: &str) {
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

        self.draw_input_dialog(f, area, 7, " New Branch ", ratatui::style::Color::Cyan, lines);
    }

    fn draw_reset_dialog(&self, f: &mut Frame, area: Rect, reset_state: &(String, String, bool)) {
        let (mode, path, selecting_mode) = reset_state;

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

        self.draw_input_dialog(f, area, 11, " Reset ", ratatui::style::Color::Red, lines);
    }

    fn draw_merge_dialog(&self, f: &mut Frame, area: Rect, merge_state: &(String, MergePreview)) {
        let (branch, preview) = merge_state;

        let files_preview = if preview.files_changed.len() > 5 {
            let first_five = preview.files_changed[..5].join(", ");
            format!("{} +{} more", first_five, preview.files_changed.len() - 5)
        } else {
            preview.files_changed.join(", ")
        };

        let lines = vec![
            Line::from(Span::styled(
                " Merge Preview ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Branch: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(branch, Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Commits: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(preview.commits_count.to_string(), Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Fast-forward: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(if preview.can_ff { "Yes" } else { "No" },
                    if preview.can_ff { Style::default().fg(ratatui::style::Color::Green) } else { Style::default().fg(ratatui::style::Color::Yellow) }),
            ]),
            Line::from(vec![
                Span::styled("  Files: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(files_preview, Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  f", Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("=Fast-forward  ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled("n", Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("=No-ff  ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled("s", Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("=Squash  ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled("q", Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("=Cancel", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]),
        ];

        self.draw_input_dialog(f, area, 12, " Merge ", ratatui::style::Color::Green, lines);
    }

    fn draw_smart_checkout_dialog(&self, f: &mut Frame, area: Rect, branch: &str) {
        let lines = vec![
            Line::from(Span::styled(
                " Smart Checkout ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Uncommitted changes detected.", Style::default().fg(ratatui::style::Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Target branch: ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(branch, Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  s", Style::default().fg(ratatui::style::Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("=Smart Checkout (shelve→checkout→unshelve)  ", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  f", Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("=Force Checkout (discard local changes)  ", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  q", Style::default().fg(ratatui::style::Color::Gray).add_modifier(Modifier::BOLD)),
                Span::styled("=Cancel", Style::default().fg(ratatui::style::Color::DarkGray)),
            ]),
        ];

        self.draw_input_dialog(f, area, 10, " Checkout ", ratatui::style::Color::Yellow, lines);
    }

    /// Draw a centered input dialog with common styling pattern.
    /// Returns the popup area used.
    fn draw_input_dialog(
        &self,
        f: &mut Frame,
        area: Rect,
        popup_h: u16,
        title: &str,
        border_color: ratatui::style::Color,
        lines: Vec<Line>,
    ) -> Rect {
        let popup_w = (area.width * 3 / 5).max(40);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(
            Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(paragraph, popup_area);
        popup_area
    }

    fn draw_rename_dialog(&self, f: &mut Frame, area: Rect, rename_state: &(String, String)) {
        let (old_name, new_name) = rename_state;

        let input_display = if new_name.is_empty() {
            "type new branch name...".to_string()
        } else {
            new_name.to_string()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Rename: ", Style::default().fg(ratatui::style::Color::Cyan)),
                Span::styled(old_name, Style::default().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  → ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(
                    input_display,
                    if new_name.is_empty() {
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
                "  Enter: rename  |  Esc: cancel",
                Style::default().fg(ratatui::style::Color::DarkGray),
            )),
        ];

        self.draw_input_dialog(f, area, 8, " Rename Branch ", ratatui::style::Color::Cyan, lines);
    }

    fn draw_ref_diff_popup(&self, f: &mut Frame, area: Rect, title: &str, content: &str, scroll: &u16) {
        let popup_w = (area.width * 4 / 5).max(40);
        let popup_h = (area.height * 4 / 5).max(10);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

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

        let title_str = format!(" Diff: {} (j/k:scroll G/g:jump PgUp/PgDn) ", title);
        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title_str.as_str())
                    .border_style(Style::default().fg(ratatui::style::Color::Yellow)),
            )
            .scroll(((*scroll), 0));

        f.render_widget(paragraph, inner);
    }

    fn draw_gitignore_popup(&self, f: &mut Frame, area: Rect, content: &str, scroll: &u16) {
        let popup_w = (area.width * 4 / 5).max(40);
        let popup_h = (area.height * 4 / 5).max(10);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

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

        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let style = if i == 0 {
                    Style::default().fg(ratatui::style::Color::Green)
                } else {
                    Style::default().fg(ratatui::style::Color::White)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" .gitignore (j/k:scroll G/g:jump PgUp/PgDn) ")
                    .border_style(Style::default().fg(ratatui::style::Color::Green)),
            )
            .scroll(((*scroll), 0));

        f.render_widget(paragraph, inner);
    }
}
