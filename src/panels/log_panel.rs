use std::any::Any;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{CommitInfo, GitError, Repo};
use crate::panels::{Action, Mode, Panel};
use crate::vimkeys::{self, parse_key_event};

pub struct LogPanel {
    commits: Vec<CommitInfo>,
    cursor: usize,
    offset: usize,
    detail_text: Vec<String>,
    diff_text: Vec<String>,
    show_full_diff: bool,
    search_visible: bool,
    search_input: String,
    search_cursor: usize,
    filtering: bool,
    filtered_indices: Vec<usize>,
}

impl LogPanel {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            cursor: 0,
            offset: 0,
            detail_text: vec!["(no commit selected)".to_string()],
            diff_text: Vec::new(),
            show_full_diff: false,
            search_visible: false,
            search_input: String::new(),
            search_cursor: 0,
            filtering: false,
            filtered_indices: Vec::new(),
        }
    }

    pub fn open_search(&mut self) {
        self.search_visible = true;
        self.search_input.clear();
        self.search_cursor = 0;
    }

    fn close_search(&mut self) {
        self.search_visible = false;
    }

    fn push_search_char(&mut self, c: char) {
        self.search_input.insert(self.search_cursor, c);
        self.search_cursor += 1;
    }

    fn search_backspace(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
            self.search_input.remove(self.search_cursor);
        }
    }

    fn apply_search(&mut self) {
        if self.search_input.is_empty() {
            self.filtering = false;
            self.filtered_indices.clear();
        } else {
            self.filtering = true;
            let query = self.search_input.to_lowercase();
            self.filtered_indices = self
                .commits
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.subject.to_lowercase().contains(&query)
                        || c.author.to_lowercase().contains(&query)
                        || c.hash.contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.cursor = 0;
        self.offset = 0;
        self.search_visible = false;
    }

    fn effective_commits(&self) -> Vec<(usize, &CommitInfo)> {
        if self.filtering {
            self.filtered_indices
                .iter()
                .filter_map(|&i| self.commits.get(i).map(|c| (i, c)))
                .collect()
        } else {
            self.commits
                .iter()
                .enumerate()
                .collect()
        }
    }

    fn effective_len(&self) -> usize {
        if self.filtering {
            self.filtered_indices.len()
        } else {
            self.commits.len()
        }
    }

    fn clamp_offset(&mut self, visible_height: usize) {
        let total = self.effective_len();
        if total == 0 {
            self.offset = 0;
            return;
        }
        if self.cursor < self.offset {
            self.offset = self.cursor;
        }
        if self.cursor >= self.offset + visible_height {
            self.offset = self.cursor - visible_height + 1;
        }
    }

    fn get_commit_at_cursor(&self) -> Option<CommitInfo> {
        let effective = self.effective_commits();
        effective.get(self.cursor).map(|(_, c)| (*c).clone())
    }

    fn load_detail(&mut self, repo: &mut Repo) {
        let commit_opt = self.get_commit_at_cursor();
        if let Some(commit) = commit_opt {
            let detail = repo.show_commit(&commit.oid).unwrap_or_default();
            self.detail_text = detail.lines().map(|l| l.to_string()).collect();
            if self.show_full_diff {
                let diff = repo.diff_commit(&commit.oid).unwrap_or_default();
                self.diff_text = diff.lines().map(|l| l.to_string()).collect();
            } else {
                self.diff_text.clear();
            }
        } else {
            self.detail_text = vec!["(no commit selected)".to_string()];
            self.diff_text.clear();
        }
    }

    pub fn set_status(&mut self, _msg: &str) {
        // Could display a status bar; for now this is a no-op placeholder
    }
}

impl Panel for LogPanel {
    fn focus(&mut self) {}
    fn blur(&mut self) {}

    fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        // Left: commit list
        let effective = self.effective_commits();
        let visible_height = cols[0].height.saturating_sub(2) as usize;
        let mut lines: Vec<Line> = Vec::new();
        let start = self.offset;
        let end = (start + visible_height).min(effective.len());
        for i in start..end {
            let (_, commit) = &effective[i];
            let is_sel = i == self.cursor;
            let style = if is_sel {
                styles.selection
            } else {
                Style::default()
            };
            let hash_span = Span::styled(
                format!("{:<8} ", commit.hash),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            let author_span = Span::styled(
                truncate_str(&commit.author, 12),
                Style::default().fg(Color::Cyan),
            );
            let date_span = Span::styled(
                format!(" {}", truncate_str(&commit.date, 10)),
                Style::default().fg(Color::DarkGray),
            );
            let subj_span = Span::styled(
                format!(" {}", truncate_str(&commit.subject, 40)),
                if is_sel { style } else { Style::default() },
            );
            lines.push(Line::from(vec![hash_span, author_span, date_span, subj_span]));
        }

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(styles.header)
            .title(" Log ");
        Paragraph::new(lines)
            .block(list_block)
            .render(cols[0], buf);

        // Right: detail + diff
        let mut detail_lines: Vec<Line> = Vec::new();
        for l in &self.detail_text {
            detail_lines.push(Line::from(Span::raw(l.as_str())));
        }
        if !self.diff_text.is_empty() {
            detail_lines.push(Line::from(""));
            for l in &self.diff_text {
                let style = if l.starts_with('+') && !l.starts_with("++") {
                    Style::default().fg(Color::Green)
                } else if l.starts_with('-') && !l.starts_with("--") {
                    Style::default().fg(Color::Red)
                } else if l.starts_with("@@") {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                };
                detail_lines.push(Line::from(Span::styled(l.as_str(), style)));
            }
        }

        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(styles.header)
            .title(if self.show_full_diff {
                " Commit Detail + Diff "
            } else {
                " Commit Detail "
            });
        Paragraph::new(detail_lines)
            .block(detail_block)
            .render(cols[1], buf);

        // Search bar overlay
        if self.search_visible {
            let search_area = Rect::new(
                area.x + 1,
                area.bottom().saturating_sub(2),
                area.width.saturating_sub(2),
                1,
            );
            let prompt = format!("/{}", &self.search_input);
            Paragraph::new(Line::from(Span::styled(
                prompt,
                Style::default().fg(Color::Yellow),
            )))
            .render(search_area, buf);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // If search is open, handle search input
        if self.search_visible {
            match key.code {
                KeyCode::Enter => {
                    self.apply_search();
                }
                KeyCode::Esc => {
                    self.close_search();
                }
                KeyCode::Backspace => {
                    self.search_backspace();
                }
                KeyCode::Char(c) => {
                    self.push_search_char(c);
                }
                _ => {}
            }
            return Some(Action::None);
        }

        let visible_height = 20usize;
        let total = self.effective_len();

        if let Some(motion) = parse_key_event(key, Mode::Normal) {
            match motion {
                vimkeys::Motion::Up(n) => {
                    self.cursor = self.cursor.saturating_sub(n);
                    self.clamp_offset(visible_height);
                }
                vimkeys::Motion::Down(n) => {
                    if total > 0 {
                        self.cursor = (self.cursor + n).min(total - 1);
                        self.clamp_offset(visible_height);
                    }
                }
                vimkeys::Motion::Top => {
                    self.cursor = 0;
                    self.offset = 0;
                }
                vimkeys::Motion::Bottom => {
                    if total > 0 {
                        self.cursor = total - 1;
                        self.clamp_offset(visible_height);
                    }
                }
                _ => {}
            }
            return Some(Action::None);
        }

        // Extract commit data at cursor before any mutations
        let cursor_commit = self.get_commit_at_cursor();

        match key.code {
            KeyCode::Enter => {
                self.show_full_diff = !self.show_full_diff;
                Some(Action::None)
            }
            KeyCode::Char('y') => {
                if let Some(commit) = cursor_commit {
                    return Some(Action::CopyHash(commit.oid));
                }
                Some(Action::None)
            }
            KeyCode::Char('x') => {
                if let Some(commit) = cursor_commit {
                    return Some(Action::CherryPick(commit.oid));
                }
                Some(Action::None)
            }
            KeyCode::Char('r') => {
                if let Some(commit) = cursor_commit {
                    return Some(Action::RebaseOnto(commit.hash));
                }
                Some(Action::None)
            }
            KeyCode::Char('/') => {
                self.open_search();
                Some(Action::SearchCommits)
            }
            KeyCode::Char('f') => {
                self.open_search();
                Some(Action::SearchCommits)
            }
            KeyCode::Tab => {
                return Some(Action::BackToMain);
            }
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Log"
    }

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        self.commits = repo.log_detailed(200)?;
        self.cursor = 0;
        self.offset = 0;
        self.load_detail(repo);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len.saturating_sub(1)].to_string();
        result.push('…');
        result
    }
}
