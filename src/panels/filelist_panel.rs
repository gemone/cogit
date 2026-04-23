use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::navigation::handle_list_navigation;
use crate::app::styles::Styles;
use crate::gitops::Repository;

pub struct FileListPanel {
    repo: std::path::PathBuf,
    focused: bool,
    pub state: ListState,
    pub files: Vec<FileItem>,
    styles: Styles,
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: String,
    pub old_path: Option<String>,
    pub status: char,
    pub staged: bool,
}

impl FileListPanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            state,
            files: Vec::new(),
            styles: styles.clone(),
        };
        panel.refresh();
        panel
    }
}

impl Panel for FileListPanel {
    fn focus(&mut self) {
        self.focused = true;
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

        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|f| {
                let status_style = match f.status {
                    'M' => self.styles.deletion,
                    'A' => self.styles.addition,
                    'D' => self.styles.deletion,
                    '?' => self.styles.text_secondary,
                    _ => self.styles.text_primary,
                };
                let prefix = if f.staged { "S " } else { "  " };
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}{} ", prefix, f.status),
                        status_style.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&f.path, self.styles.text_primary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Files ")
                    .border_style(border_style),
            )
            .highlight_style(self.styles.highlight);

        f.render_stateful_widget(list, area, &mut self.state);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle navigation with the shared helper
        if handle_list_navigation(&mut self.state, self.files.len(), key.code) {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let i = self.state.selected().unwrap_or(0);
                if let Some(file) = self.files.get(i) {
                    return Some(Action::ShowDiff(file.path.clone()));
                }
                None
            }
            KeyCode::Char(' ') => {
                let i = self.state.selected().unwrap_or(0);
                if let Some(file) = self.files.get(i) {
                    if file.staged {
                        return Some(Action::Unstage);
                    } else {
                        return Some(Action::Stage);
                    }
                }
                None
            }
            KeyCode::Char('a') => Some(Action::StageAll),
            KeyCode::Char('c') => Some(Action::CommitDialog),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Reset dialog for selected file (mixed mode unstages)
                Some(Action::ResetDialog("mixed".to_string()))
            }
            KeyCode::Char('u') => Some(Action::UnstageAll),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Files"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo)
            && let Ok(status) = repo.status() {
                self.files.clear();
                for f in &status.staged {
                    self.files.push(FileItem {
                        path: f.path.clone(),
                        old_path: None,
                        status: f.status,
                        staged: true,
                    });
                }
                for f in &status.unstaged {
                    self.files.push(FileItem {
                        path: f.path.clone(),
                        old_path: None,
                        status: f.status,
                        staged: false,
                    });
                }
                for f in &status.untracked {
                    self.files.push(FileItem {
                        path: f.path.clone(),
                        old_path: None,
                        status: f.status,
                        staged: false,
                    });
                }
            }
        if self.files.is_empty() {
            self.state.select(None);
        } else if self.state.selected().unwrap_or(0) >= self.files.len() {
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
