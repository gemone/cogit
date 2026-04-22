use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::styles::Styles;
use crate::gitops::Repository;
use crate::gitops::types::CommitDetail;

pub struct LogPanel {
    repo: std::path::PathBuf,
    focused: bool,
    state: ListState,
    commits: Vec<crate::gitops::types::CommitInfo>,
    selected_detail: Option<CommitDetail>,
    styles: Styles,
    search_mode: bool,
    search_query: String,
}

impl LogPanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            state,
            commits: Vec::new(),
            selected_detail: None,
            styles: styles.clone(),
            search_mode: false,
            search_query: String::new(),
        };
        panel.refresh();
        panel
    }

    fn selected_hash(&self) -> Option<String> {
        let i = self.state.selected().unwrap_or(0);
        self.commits.get(i).map(|c| c.hash.clone())
    }
}

impl Panel for LogPanel {
    fn focus(&mut self) {
        self.focused = true;
        self.refresh();
    }

    fn blur(&mut self) {
        self.focused = false;
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            self.styles.border_active
        } else {
            self.styles.border_inactive
        };

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Commit list
        let title = if self.search_mode {
            format!(" Log [search: {}] ", self.search_query)
        } else {
            " Log ".to_string()
        };

        let items: Vec<ListItem> = self
            .commits
            .iter()
            .map(|c| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", c.short_hash),
                        self.styles.addition.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&c.subject, self.styles.text_primary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .highlight_style(self.styles.highlight);

        f.render_stateful_widget(list, chunks[0], &mut self.state);

        // Commit detail
        let detail_text = if let Some(ref detail) = self.selected_detail {
            let info = &detail.info;
            vec![
                Line::from(vec![
                    Span::styled("Hash: ", self.styles.text_secondary),
                    Span::styled(&info.hash, self.styles.text_primary),
                ]),
                Line::from(vec![
                    Span::styled("Author: ", self.styles.text_secondary),
                    Span::styled(
                        format!("{} <{}>", info.author_name, info.author_email),
                        self.styles.text_primary,
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Date: ", self.styles.text_secondary),
                    Span::styled(&info.date, self.styles.text_primary),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(&info.subject, self.styles.text_primary)]),
                Line::from(detail.body.clone()),
            ]
        } else {
            vec![Line::from("Select a commit")]
        };

        let detail_widget = Paragraph::new(detail_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Detail ")
                    .border_style(border_style),
            )
            .scroll((0, 0));

        f.render_widget(detail_widget, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.refresh();
                    return None;
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    return None;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.refresh();
                    return None;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.refresh();
                    return None;
                }
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.commits.len();
                if len > 0 {
                    let i = self.state.selected().unwrap_or(0);
                    let new_i = (i + 1).min(len - 1);
                    self.state.select(Some(new_i));
                    self.load_detail();
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some(i.saturating_sub(1)));
                self.load_detail();
                None
            }
            KeyCode::Char('G') => {
                if !self.commits.is_empty() {
                    self.state.select(Some(self.commits.len() - 1));
                    self.load_detail();
                }
                None
            }
            KeyCode::Char('g') => {
                self.state.select(Some(0));
                self.load_detail();
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some(i.saturating_sub(10)));
                self.load_detail();
                None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let len = self.commits.len();
                if len > 0 {
                    let i = self.state.selected().unwrap_or(0);
                    self.state.select(Some((i + 10).min(len - 1)));
                    self.load_detail();
                }
                None
            }
            KeyCode::Char('y') => {
                if let Some(hash) = self.selected_hash() {
                    Some(Action::CopyHash(hash))
                } else {
                    None
                }
            }
            KeyCode::Char('c') => {
                if let Some(hash) = self.selected_hash() {
                    Some(Action::CherryPick(hash))
                } else {
                    None
                }
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Log"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo) {
            if self.search_query.is_empty() {
                self.commits = repo.log(100).unwrap_or_default();
            } else {
                self.commits = repo.log_search(&self.search_query, 100).unwrap_or_default();
            }
        }
        if self.commits.is_empty() {
            self.state.select(None);
        } else if self.state.selected().unwrap_or(0) >= self.commits.len() {
            self.state.select(Some(0));
        }
        self.load_detail();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl LogPanel {
    fn load_detail(&mut self) {
        if let Some(hash) = self.selected_hash() {
            if let Ok(repo) = Repository::open(&self.repo) {
                self.selected_detail = repo.show_commit(&hash).ok();
            }
        } else {
            self.selected_detail = None;
        }
    }
}
