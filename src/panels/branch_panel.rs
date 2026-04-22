use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::styles::Styles;
use crate::gitops::Repository;

pub struct BranchPanel {
    repo: std::path::PathBuf,
    focused: bool,
    state: ListState,
    branches: Vec<crate::gitops::types::BranchInfo>,
    styles: Styles,
    search_mode: bool,
    search_query: String,
    filtered_indices: Vec<usize>,
}

impl BranchPanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            state,
            branches: Vec::new(),
            styles: styles.clone(),
            search_mode: false,
            search_query: String::new(),
            filtered_indices: Vec::new(),
        };
        panel.refresh();
        panel
    }

    fn current_branch_name(&self) -> Option<String> {
        let i = self.state.selected().unwrap_or(0);
        let actual_i = self.filtered_indices.get(i).copied().unwrap_or(i);
        self.branches.get(actual_i).map(|b| b.name.clone())
    }
}

impl Panel for BranchPanel {
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

        let title = if self.search_mode {
            format!(" Branches [search: {}] ", self.search_query)
        } else {
            " Branches ".to_string()
        };

        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.branches.get(idx))
            .map(|b| {
                let prefix = if b.is_current { "* " } else { "  " };
                let name_style = if b.is_current {
                    self.styles.addition.add_modifier(Modifier::BOLD)
                } else if b.is_remote {
                    self.styles.text_secondary
                } else {
                    self.styles.text_primary
                };
                let line = Line::from(vec![
                    Span::styled(prefix.to_string(), name_style),
                    Span::styled(&b.name, name_style),
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

        f.render_stateful_widget(list, area, &mut self.state);

        // Help text at bottom
        let help = Paragraph::new(
            "Enter:switch n:new d:delete f:fetch p:push P:pull m:merge r:rebase /:search q:back",
        )
        .style(self.styles.text_secondary);
        let help_area = Rect {
            y: area.bottom().saturating_sub(1),
            height: 1,
            ..area
        };
        f.render_widget(help, help_area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.apply_filter();
                    return None;
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    return None;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.apply_filter();
                    return None;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.apply_filter();
                    return None;
                }
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.filtered_indices.len();
                if len > 0 {
                    let i = self.state.selected().unwrap_or(0);
                    self.state.select(Some((i + 1).min(len - 1)));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some(i.saturating_sub(1)));
                None
            }
            KeyCode::Char('G') => {
                if !self.filtered_indices.is_empty() {
                    self.state.select(Some(self.filtered_indices.len() - 1));
                }
                None
            }
            KeyCode::Char('g') => {
                self.state.select(Some(0));
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some(i.saturating_sub(10)));
                None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let len = self.filtered_indices.len();
                if len > 0 {
                    let i = self.state.selected().unwrap_or(0);
                    self.state.select(Some((i + 10).min(len - 1)));
                }
                None
            }
            KeyCode::Enter => {
                self.current_branch_name().map(Action::CheckoutBranch)
            }
            KeyCode::Char('n') => {
                // Create branch - simplified: prompt via Action
                Some(Action::CreateBranch("new-branch".to_string()))
            }
            KeyCode::Char('d') => {
                self.current_branch_name().map(Action::DeleteBranch)
            }
            KeyCode::Char('f') => Some(Action::FetchAll),
            KeyCode::Char('p') => Some(Action::PushCurrent),
            KeyCode::Char('P') => Some(Action::PullCurrent),
            KeyCode::Char('m') => {
                self.current_branch_name().map(Action::MergeBranch)
            }
            KeyCode::Char('r') => {
                self.current_branch_name().map(Action::RebaseBranch)
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
        "Branches"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo) {
            self.branches = repo.branches().unwrap_or_default();
        }
        self.apply_filter();
        if self.filtered_indices.is_empty() {
            self.state.select(None);
        } else if self.state.selected().unwrap_or(0) >= self.filtered_indices.len() {
            self.state.select(Some(0));
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl BranchPanel {
    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.branches.len()).collect();
        } else {
            let q = self.search_query.to_lowercase();
            self.filtered_indices = self
                .branches
                .iter()
                .enumerate()
                .filter(|(_, b)| b.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();
        }
    }
}
