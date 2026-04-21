use std::any::Any;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo};
use crate::panels::{Action, Mode, Panel};

// ── Data ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchSection {
    Local,
    Remote,
    Tags,
}

#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub section: BranchSection,
    pub is_current: bool,
    pub last_msg: String,
    pub date: String,
    pub remote_name: Option<String>,
}

// ── Dialog state ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchDialog {
    None,
    SmartCheckout,   // "Stash & checkout", "Force checkout", "Cancel"
    NewBranch,
    RenameBranch,
    DeleteConfirm,
    PullChoice,      // "Merge", "Rebase", "Cancel"
}

// ── Panel ─────────────────────────────────────────────────────────────

pub struct BranchPanel {
    focused: bool,
    entries: Vec<BranchEntry>,
    cursor: usize,
    offset: usize,
    dialog: BranchDialog,
    dialog_cursor: usize,
    input: String,
    status_msg: String,
}

impl BranchPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            entries: Vec::new(),
            cursor: 0,
            offset: 0,
            dialog: BranchDialog::None,
            dialog_cursor: 0,
            input: String::new(),
            status_msg: String::new(),
        }
    }

    fn selected_entry(&self) -> Option<&BranchEntry> {
        self.entries.get(self.cursor)
    }

    fn clamp_offset(&mut self, visible: usize) {
        if self.cursor < self.offset {
            self.offset = self.cursor;
        }
        if visible > 0 && self.cursor >= self.offset + visible {
            self.offset = self.cursor.saturating_sub(visible - 1);
        }
    }

    fn move_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
        self.clamp_offset(20);
    }

    fn move_down(&mut self, n: usize) {
        if !self.entries.is_empty() {
            self.cursor = (self.cursor + n).min(self.entries.len() - 1);
            self.clamp_offset(20);
        }
    }

    pub fn show_smart_checkout(&mut self, _name: &str) {
        self.dialog = BranchDialog::SmartCheckout;
        self.dialog_cursor = 0;
    }

    pub fn show_new_branch_dialog(&mut self) {
        self.dialog = BranchDialog::NewBranch;
        self.dialog_cursor = 0;
        self.input.clear();
    }

    pub fn show_pull_dialog(&mut self) {
        self.dialog = BranchDialog::PullChoice;
        self.dialog_cursor = 0;
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_msg = msg.to_string();
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        self.selected_entry()
            .filter(|e| e.section == BranchSection::Local)
            .map(|e| e.name.clone())
    }

    fn handle_dialog_keys(&mut self, key: KeyEvent) -> Option<Action> {
        match self.dialog {
            BranchDialog::SmartCheckout => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.dialog_cursor = self.dialog_cursor.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.dialog_cursor = (self.dialog_cursor + 1).min(2);
                }
                KeyCode::Enter => {
                    let name = self
                        .selected_entry()
                        .map(|e| e.name.clone())
                        .unwrap_or_default();
                    self.dialog = BranchDialog::None;
                    match self.dialog_cursor {
                        0 => return Some(Action::Stash),
                        1 => return Some(Action::ForceCheckout(name)),
                        _ => {}
                    }
                }
                KeyCode::Esc => {
                    self.dialog = BranchDialog::None;
                }
                _ => {}
            },
            BranchDialog::NewBranch => match key.code {
                KeyCode::Enter => {
                    let name = self.input.trim().to_string();
                    self.dialog = BranchDialog::None;
                    self.input.clear();
                    if !name.is_empty() {
                        return Some(Action::CreateBranch(name));
                    }
                }
                KeyCode::Esc => {
                    self.dialog = BranchDialog::None;
                    self.input.clear();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                _ => {}
            },
            BranchDialog::RenameBranch => match key.code {
                KeyCode::Enter => {
                    let new_name = self.input.trim().to_string();
                    let old_name = self.selected_entry().map(|e| e.name.clone()).unwrap_or_default();
                    self.dialog = BranchDialog::None;
                    self.input.clear();
                    if !new_name.is_empty() {
                        return Some(Action::RenameBranch { old_name, new_name });
                    }
                }
                KeyCode::Esc => {
                    self.dialog = BranchDialog::None;
                    self.input.clear();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                _ => {}
            },
            BranchDialog::DeleteConfirm => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let name = self.selected_entry().map(|e| e.name.clone()).unwrap_or_default();
                    self.dialog = BranchDialog::None;
                    if !name.is_empty() {
                        return Some(Action::DeleteBranchConfirm(name));
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.dialog = BranchDialog::None;
                }
                _ => {}
            },
            BranchDialog::PullChoice => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.dialog_cursor = self.dialog_cursor.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.dialog_cursor = (self.dialog_cursor + 1).min(2);
                }
                KeyCode::Enter => {
                    self.dialog = BranchDialog::None;
                    match self.dialog_cursor {
                        0 => return Some(Action::PullMerge),
                        1 => return Some(Action::PullRebase),
                        _ => {}
                    }
                }
                KeyCode::Esc => {
                    self.dialog = BranchDialog::None;
                }
                _ => {}
            },
            BranchDialog::None => {}
        }
        None
    }

    fn render_dialog(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        let (title, options): (&str, Vec<&str>) = match self.dialog {
            BranchDialog::SmartCheckout => (
                " Smart Checkout ",
                vec!["Stash & checkout", "Force checkout", "Cancel"],
            ),
            BranchDialog::NewBranch => {
                return self.render_input_dialog(area, buf, styles, " New Branch ", "Branch name:");
            }
            BranchDialog::RenameBranch => {
                return self.render_input_dialog(area, buf, styles, " Rename Branch ", "New name:");
            }
            BranchDialog::DeleteConfirm => {
                let name = self.selected_entry().map(|e| e.name.as_str()).unwrap_or("?");
                let msg = format!("Delete branch \"{}\"? (y/n)", name);
                let popup_area = centered_rect(50, 3, area);
                Clear.render(popup_area, buf);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(styles.conflict)
                    .title(" Confirm Delete ");
                let para = Paragraph::new(msg).block(block);
                para.render(popup_area, buf);
                return;
            }
            BranchDialog::PullChoice => (
                " Pull ",
                vec!["Merge", "Rebase", "Cancel"],
            ),
            BranchDialog::None => return,
        };

        let height = options.len() as u16 + 2;
        let popup_area = centered_rect(40, height, area);
        Clear.render(popup_area, buf);

        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let style = if i == self.dialog_cursor {
                    styles.selection
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(format!(" {}", opt), style)))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(styles.header)
            .title(title);

        List::new(items)
            .block(block)
            .render(popup_area, buf);
    }

    fn render_input_dialog(
        &self,
        area: Rect,
        buf: &mut Buffer,
        styles: &Styles,
        title: &str,
        prompt: &str,
    ) {
        let popup_area = centered_rect(50, 5, area);
        Clear.render(popup_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(styles.header)
            .title(title);

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let lines = vec![
            Line::from(Span::styled(prompt, styles.cmdbar)),
            Line::from(Span::raw(&self.input)),
            Line::from(Span::styled("Enter=confirm  Esc=cancel", Style::default().fg(Color::DarkGray))),
        ];
        let para = Paragraph::new(lines);
        para.render(inner, buf);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, popup_width.min(r.width), height.min(r.height))
}

// ── Panel impl ────────────────────────────────────────────────────────

impl Panel for BranchPanel {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn blur(&mut self) {
        self.focused = false;
    }

    fn title(&self) -> &str {
        "branches"
    }

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        self.entries.clear();

        // Local branches
        let local_output = shell::run_git(&mut shell::git_cmd(&repo.path).args([
            "branch",
            "--format=%(refname:short)%(if)%(HEAD)%(then) *CURRENT*%(end)",
        ]))?;

        for line in local_output.lines() {
            if line.is_empty() {
                continue;
            }
            let is_current = line.contains("*CURRENT*");
            let name = line.replace(" *CURRENT*", "");
            let (msg, date) = get_branch_info(&repo.path, &name);
            self.entries.push(BranchEntry {
                name,
                section: BranchSection::Local,
                is_current,
                last_msg: msg,
                date,
                remote_name: None,
            });
        }

        // Remote branches
        let remote_output = shell::run_git(&mut shell::git_cmd(&repo.path).args([
            "branch",
            "-r",
            "--format=%(refname:short)",
        ]))?;

        for name in remote_output.lines() {
            if name.is_empty() || name.contains(" -> ") {
                continue;
            }
            let (msg, date) = get_branch_info(&repo.path, name);
            let remote_name = name.split('/').next().map(|s| s.to_string());
            self.entries.push(BranchEntry {
                name: name.to_string(),
                section: BranchSection::Remote,
                is_current: false,
                last_msg: msg,
                date,
                remote_name,
            });
        }

        // Tags
        let tags_output = shell::list_tags(&repo.path).unwrap_or_default();
        for tag in tags_output.lines() {
            if tag.is_empty() {
                continue;
            }
            let (msg, date) = get_tag_info(&repo.path, tag);
            self.entries.push(BranchEntry {
                name: tag.to_string(),
                section: BranchSection::Tags,
                is_current: false,
                last_msg: msg,
                date,
                remote_name: None,
            });
        }

        if self.cursor >= self.entries.len() && !self.entries.is_empty() {
            self.cursor = self.entries.len() - 1;
        }
        Ok(())
    }

    fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        let border = if self.focused {
            styles.border_active
        } else {
            styles.border_inactive
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(" Branches ");

        if self.entries.is_empty() {
            Paragraph::new("(no branches)")
                .block(block)
                .render(area, buf);
            return;
        }

        let inner = block.inner(area);
        let visible = inner.height as usize;

        let mut render_offset = self.offset;
        if self.cursor < render_offset {
            render_offset = self.cursor;
        }
        if visible > 0 && self.cursor >= render_offset + visible {
            render_offset = self.cursor.saturating_sub(visible - 1);
        }

        let end = (render_offset + visible).min(self.entries.len());
        let slice = if render_offset < self.entries.len() {
            &self.entries[render_offset..end]
        } else {
            &[]
        };

        let mut current_section: Option<BranchSection> = None;
        let header_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let mut items: Vec<ListItem> = Vec::new();
        for (vi, entry) in slice.iter().enumerate() {
            let actual_idx = render_offset + vi;
            if current_section != Some(entry.section) {
                current_section = Some(entry.section);
                let label = match entry.section {
                    BranchSection::Local => "Local",
                    BranchSection::Remote => "Remote",
                    BranchSection::Tags => "Tags",
                };
                items.push(ListItem::new(Line::from(Span::styled(
                    format!(" {} ", label),
                    header_style,
                ))));
            }
            let is_sel = actual_idx == self.cursor && self.focused;
            let marker = if entry.is_current { "● " } else { "  " };
            let msg = truncate_str(&entry.last_msg, 40);
            let line = Line::from(vec![
                Span::styled(marker.to_string(), if entry.is_current { styles.addition } else { Style::default() }),
                Span::styled(entry.name.clone(), if is_sel { styles.selection } else { Style::default() }),
                Span::raw("  "),
                Span::styled(msg, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(entry.date.clone(), Style::default().fg(Color::DarkGray)),
            ]);
            items.push(ListItem::new(line));
        }

        List::new(items).block(block).render(area, buf);

        // Status message
        if !self.status_msg.is_empty() {
            let status_area = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
            let msg = Paragraph::new(Line::from(Span::styled(
                &self.status_msg,
                styles.header,
            )));
            msg.render(status_area, buf);
        }

        // Dialog overlay
        self.render_dialog(area, buf, styles);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Dialog takes priority
        if self.dialog != BranchDialog::None {
            return self.handle_dialog_keys(key);
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(1),
            KeyCode::Char('G') => {
                if !self.entries.is_empty() {
                    self.cursor = self.entries.len() - 1;
                    self.clamp_offset(20);
                }
            }
            KeyCode::Char('g') => {
                self.cursor = 0;
                self.clamp_offset(20);
            }

            KeyCode::Enter | KeyCode::Char('c') => {
                if let Some(entry) = self.selected_entry() {
                    if entry.section == BranchSection::Local && !entry.is_current {
                        return Some(Action::CheckoutSmart(entry.name.clone()));
                    }
                }
            }
            KeyCode::Char('n') => {
                self.dialog = BranchDialog::NewBranch;
                self.input.clear();
            }
            KeyCode::Char('D') => {
                if let Some(entry) = self.selected_entry() {
                    if entry.section == BranchSection::Local && !entry.is_current {
                        self.dialog = BranchDialog::DeleteConfirm;
                    }
                }
            }
            KeyCode::Char('r') => {
                if self.selected_entry().map(|e| e.section == BranchSection::Local).unwrap_or(false) {
                    self.dialog = BranchDialog::RenameBranch;
                    self.input.clear();
                }
            }
            KeyCode::Char('f') => {
                let remote = self
                    .selected_entry()
                    .and_then(|e| e.remote_name.clone())
                    .unwrap_or_else(|| "origin".to_string());
                return Some(Action::FetchRemote(remote));
            }
            KeyCode::Char('p') => {
                return Some(Action::PushCurrent);
            }
            KeyCode::Char('P') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.dialog = BranchDialog::PullChoice;
                self.dialog_cursor = 0;
            }
            KeyCode::Char('m') => {
                if let Some(entry) = self.selected_entry() {
                    if entry.section == BranchSection::Local || entry.section == BranchSection::Remote {
                        return Some(Action::MergeBranch(entry.name.clone()));
                    }
                }
            }
            KeyCode::Char('R') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(entry) = self.selected_entry() {
                    return Some(Action::RebaseOnto(entry.name.clone()));
                }
            }
            KeyCode::Tab => {
                return Some(Action::BackToMain);
            }
            KeyCode::Char('u') => {
                // set upstream — for now just push with -u
                return Some(Action::PushCurrent);
            }
            _ => {}
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

use crate::gitops::shell;

fn get_branch_info(repo_path: &std::path::Path, name: &str) -> (String, String) {
    let msg = shell::run_git(&mut shell::git_cmd(repo_path)
        .args(["log", "-1", "--format=%s", name]))
        .unwrap_or_default();
    let date = shell::run_git(&mut shell::git_cmd(repo_path)
        .args(["log", "-1", "--format=%cr", name]))
        .unwrap_or_default();
    (truncate_str(&msg, 60), date)
}

fn get_tag_info(repo_path: &std::path::Path, tag: &str) -> (String, String) {
    let msg = shell::run_git(&mut shell::git_cmd(repo_path)
        .args(["log", "-1", "--format=%s", tag]))
        .unwrap_or_default();
    let date = shell::run_git(&mut shell::git_cmd(repo_path)
        .args(["log", "-1", "--format=%cr", tag]))
        .unwrap_or_default();
    (truncate_str(&msg, 60), date)
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
