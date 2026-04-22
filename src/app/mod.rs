pub mod cmdline;
pub mod notification;
pub mod styles;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::Path;

use crate::gitops::Repository;
use crate::panels::{
    branch_panel::BranchPanel, filelist_panel::FileListPanel, log_panel::LogPanel,
    stash_panel::StashPanel, Action, Panel,
};
use crate::vimkeys::Mode;

use self::cmdline::CmdLine;
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
    diff_popup: Option<(String, u16)>, // (content, scroll)
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
        })
    }

    pub fn run(&mut self, terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.draw_frame(f))?;
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_event(key);
                }
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
                    if let Some((_, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_add(1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some((_, ref mut scroll)) = self.diff_popup {
                        *scroll = scroll.saturating_sub(1);
                    }
                }
                _ => {}
            }
            return;
        }

        // Command mode takes priority
        if self.cmdline.is_visible() {
            self.handle_command_key(key);
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
            KeyCode::Char('s') => {
                if let Some(action) = self.filelist.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)) {
                    self.dispatch(action);
                }
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

        let action = match cmd {
            ":w" | ":stage" => Action::Stage,
            ":wq" => Action::CommitDialog,
            ":q" | ":q!" => Action::Quit,
            ":stageall" => Action::StageAll,
            ":unstage" => Action::Unstage,
            ":unstageall" => Action::UnstageAll,
            ":commit" | ":c" => Action::CommitDialog,
            ":discard" => Action::Discard,
            ":stash" => Action::ShowStashPanel,
            ":stashpop" => Action::StashPop(0),
            ":push" => Action::PushCurrent,
            ":fetch" => Action::FetchAll,
            ":branch" | ":branches" => Action::ShowBranchPanel,
            ":log" => Action::ShowLogPanel,
            ":help" => Action::Help,
            ":amend" => Action::AmendCommit,
            "" => return,
            _ => {
                self.notifications.notify_error(&format!("Unknown command: {}", cmd));
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
                    self.notifications.notify_error(&format!("Stage all failed: {}", e));
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
                    self.notifications.notify_error(&format!("Unstage all failed: {}", e));
                } else {
                    self.notifications.notify("All files unstaged");
                    self.refresh_all();
                }
            }
            Action::Discard => {
                self.dispatch_discard();
            }
            Action::CommitDialog => {
                // Simplified: commit with default message
                self.notifications.notify("Use :commit -m \"message\" to commit");
            }
            Action::Commit(msg) => {
                match self.repo.commit(&msg) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Committed: {}", msg));
                        self.refresh_all();
                    }
                    Err(e) => self.notifications.notify_error(&format!("Commit failed: {}", e)),
                }
            }
            Action::AmendCommit => {
                match self.repo.amend_commit(None) {
                    Ok(_) => {
                        self.notifications.notify("Commit amended");
                        self.refresh_all();
                    }
                    Err(e) => self.notifications.notify_error(&format!("Amend failed: {}", e)),
                }
            }
            Action::CheckoutBranch(name) => {
                match self.repo.checkout(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Switched to {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Checkout failed: {}", e));
                    }
                }
            }
            Action::PushCurrent => {
                match self.repo.push_current() {
                    Ok(_) => {
                        self.notifications.notify("Pushed successfully");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Push failed: {}", e));
                    }
                }
            }
            Action::FetchAll => {
                match self.repo.fetch_all() {
                    Ok(_) => {
                        self.notifications.notify("Fetched all remotes");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Fetch failed: {}", e));
                    }
                }
            }
            Action::PullCurrent => {
                match self.repo.pull_current() {
                    Ok(_) => {
                        self.notifications.notify("Pulled successfully");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Pull failed: {}", e));
                    }
                }
            }
            Action::CreateBranch(name) => {
                match self.repo.create_branch(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Created branch: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Create branch failed: {}", e));
                    }
                }
            }
            Action::DeleteBranch(name) => {
                match self.repo.delete_branch(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Deleted branch: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Delete branch failed: {}", e));
                    }
                }
            }
            Action::MergeBranch(name) => {
                match self.repo.merge(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Merged: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Merge failed: {}", e));
                    }
                }
            }
            Action::RebaseBranch(name) => {
                match self.repo.rebase(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Rebased onto: {}", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Rebase failed: {}", e));
                    }
                }
            }
            Action::CherryPick(hash) => {
                match self.repo.cherry_pick(&hash) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Cherry-picked: {}", &hash[..7.min(hash.len())]));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Cherry-pick failed: {}", e));
                    }
                }
            }
            Action::CopyHash(hash) => {
                // Copy to clipboard - simplified notification
                self.notifications.notify(&format!("Copied: {}", &hash[..7.min(hash.len())]));
            }
            Action::SearchLog(_query) => {
                // Handled by log panel directly
            }
            Action::Stash => {
                match self.repo.stash_create(None) {
                    Ok(_) => {
                        self.notifications.notify("Stash created");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Stash failed: {}", e));
                    }
                }
            }
            Action::StashPop(index) => {
                match self.repo.stash_pop(index) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Stash@{} popped", index));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Stash pop failed: {}", e));
                    }
                }
            }
            Action::StashApply(index) => {
                match self.repo.stash_apply(index) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Stash@{} applied", index));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Stash apply failed: {}", e));
                    }
                }
            }
            Action::StashDrop(index) => {
                match self.repo.stash_drop(index) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Stash@{} dropped", index));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Stash drop failed: {}", e));
                    }
                }
            }
            Action::ShelveApply(name) => {
                match self.repo.shelve_apply(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Shelve '{}' applied", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Shelve apply failed: {}", e));
                    }
                }
            }
            Action::ShelveDrop(name) => {
                match self.repo.shelve_drop(&name) {
                    Ok(_) => {
                        self.notifications.notify(&format!("Shelve '{}' deleted", name));
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Shelve drop failed: {}", e));
                    }
                }
            }
            Action::ShelveCreate => {
                match self.repo.shelve_create("default") {
                    Ok(_) => {
                        self.notifications.notify("Shelve created");
                        self.refresh_all();
                    }
                    Err(e) => {
                        self.notifications.notify_error(&format!("Shelve create failed: {}", e));
                    }
                }
            }
            Action::Help => {
                self.notifications
                    .notify("1:branches 2:log 4:stash :commands q:quit");
            }
            Action::ShowDiff(path) => {
                let content = self.repo.file_diff(&path).unwrap_or_else(|e| format!("Error: {}", e));
                self.diff_popup = Some((content, 0));
            }
        }
    }

    fn dispatch_stage(&mut self) {
        // Stage the currently selected file
        let i = self.filelist.state.selected().unwrap_or(0);
        if let Some(file) = self.filelist.files.get(i) {
            if let Err(e) = self.repo.stage(&file.path) {
                self.notifications.notify_error(&format!("Stage failed: {}", e));
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
                self.notifications.notify_error(&format!("Unstage failed: {}", e));
            } else {
                self.notifications.notify(&format!("Unstaged: {}", file.path));
                self.refresh_all();
            }
        }
    }

    fn dispatch_discard(&mut self) {
        let i = self.filelist.state.selected().unwrap_or(0);
        if let Some(file) = self.filelist.files.get(i) {
            if let Err(e) = self.repo.discard(&file.path) {
                self.notifications.notify_error(&format!("Discard failed: {}", e));
            } else {
                self.notifications.notify(&format!("Discarded: {}", file.path));
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
        if let Some((ref content, scroll)) = self.diff_popup {
            self.draw_diff_popup(f, size, content, scroll);
        }

        // Notifications on top of everything
        self.notifications.render(f, size);
    }

    fn draw_main(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status bar
                Constraint::Min(5),     // File list
                Constraint::Length(1),  // Sidebar help
            ])
            .split(area);

        // Status bar
        let branch = self.repo.current_branch().unwrap_or_default();
        let status_bar = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", branch),
                self.styles.addition.add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " cogit ",
                self.styles.text_secondary,
            ),
        ]))
        .style(Style::default().bg(ratatui::style::Color::DarkGray));
        f.render_widget(status_bar, chunks[0]);

        // File list
        self.filelist.focus();
        self.filelist.render(f, chunks[1]);
        let help = Paragraph::new(
            " 1:branches 2:log 4:stash :commands s:stage S:stage-all c:commit q:quit",
        )
        .style(self.styles.text_secondary);
        f.render_widget(help, chunks[2]);
    }

    fn draw_diff_popup(&self, f: &mut Frame, area: Rect, content: &str, scroll: u16) {
        // Centered popup: 80% width, 80% height
        let popup_w = (area.width as u16 * 4 / 5).max(40);
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

        let title = " Diff (Esc:close j/k:scroll) ";
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
}
