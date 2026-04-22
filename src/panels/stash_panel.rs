use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::styles::Styles;
use crate::gitops::{Repository, shelve::ShelveEntry, stash::StashEntry};

#[derive(Debug, Clone, PartialEq)]
enum StashTab {
    Stash,
    Shelve,
}

pub struct StashPanel {
    repo: std::path::PathBuf,
    focused: bool,
    styles: Styles,
    tab: StashTab,
    // Stash state
    stash_state: ListState,
    stash_entries: Vec<StashEntry>,
    // Shelve state
    shelve_state: ListState,
    shelve_entries: Vec<ShelveEntry>,
}

impl StashPanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut stash_state = ListState::default();
        stash_state.select(Some(0));
        let mut shelve_state = ListState::default();
        shelve_state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            styles: styles.clone(),
            tab: StashTab::Stash,
            stash_state,
            stash_entries: Vec::new(),
            shelve_state,
            shelve_entries: Vec::new(),
        };
        panel.refresh();
        panel
    }

    fn selected_stash_index(&self) -> Option<usize> {
        let i = self.stash_state.selected().unwrap_or(0);
        self.stash_entries.get(i).map(|e| e.index)
    }

    fn selected_shelve_name(&self) -> Option<String> {
        let i = self.shelve_state.selected().unwrap_or(0);
        self.shelve_entries.get(i).map(|e| e.name.clone())
    }
}

impl Panel for StashPanel {
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
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Help
            ])
            .split(area);

        // Render tabs
        let tab_titles = vec![
            Span::styled(
                " Stash ",
                if self.tab == StashTab::Stash {
                    self.styles.addition.add_modifier(Modifier::BOLD)
                } else {
                    self.styles.text_secondary
                },
            ),
            Span::styled(
                " Shelve ",
                if self.tab == StashTab::Shelve {
                    self.styles.addition.add_modifier(Modifier::BOLD)
                } else {
                    self.styles.text_secondary
                },
            ),
        ];
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Stash/Shelve ")
                    .border_style(border_style),
            )
            .select(if self.tab == StashTab::Stash { 0 } else { 1 });
        f.render_widget(tabs, chunks[0]);

        // Render content based on tab
        match self.tab {
            StashTab::Stash => self.render_stash(f, chunks[1], border_style),
            StashTab::Shelve => self.render_shelve(f, chunks[1], border_style),
        }

        // Render help
        let help_text = match self.tab {
            StashTab::Stash => "Enter:pop a:apply d:drop s:stash Tab:shelve q:back",
            StashTab::Shelve => "Enter:apply d:delete Tab:stash q:back",
        };
        let help = Paragraph::new(help_text).style(self.styles.text_secondary);
        f.render_widget(help, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Tab => {
                self.tab = if self.tab == StashTab::Stash {
                    StashTab::Shelve
                } else {
                    StashTab::Stash
                };
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => match self.tab {
                StashTab::Stash => self.handle_stash_key(key),
                StashTab::Shelve => self.handle_shelve_key(key),
            },
        }
    }

    fn title(&self) -> &str {
        "Stash/Shelve"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo) {
            self.stash_entries = repo.stash_list().unwrap_or_default();
            self.shelve_entries = repo.list_shelves().unwrap_or_default();
        }
        // Fix selections
        if self.stash_entries.is_empty() {
            self.stash_state.select(None);
        } else {
            let i = self.stash_state.selected().unwrap_or(0);
            if i >= self.stash_entries.len() {
                self.stash_state.select(Some(0));
            }
        }
        if self.shelve_entries.is_empty() {
            self.shelve_state.select(None);
        } else {
            let i = self.shelve_state.selected().unwrap_or(0);
            if i >= self.shelve_entries.len() {
                self.shelve_state.select(Some(0));
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl StashPanel {
    fn render_stash(&mut self, f: &mut Frame, area: Rect, border_style: Style) {
        let items: Vec<ListItem> = self
            .stash_entries
            .iter()
            .map(|e| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("stash@{{{}}} ", e.index),
                        self.styles.addition.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&e.message, self.styles.text_primary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let empty_msg = if self.stash_entries.is_empty() {
            "No stash entries"
        } else {
            ""
        };

        let list = List::new(if items.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                empty_msg.to_string(),
                self.styles.text_secondary,
            )))]
        } else {
            items
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Stash Entries ")
                .border_style(border_style),
        )
        .highlight_style(self.styles.highlight);

        f.render_stateful_widget(list, area, &mut self.stash_state);
    }

    fn render_shelve(&mut self, f: &mut Frame, area: Rect, border_style: Style) {
        let items: Vec<ListItem> = self
            .shelve_entries
            .iter()
            .map(|e| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", e.name),
                        self.styles.addition.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&e.date, self.styles.text_secondary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let empty_msg = if self.shelve_entries.is_empty() {
            "No shelve entries"
        } else {
            ""
        };

        let list = List::new(if items.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                empty_msg.to_string(),
                self.styles.text_secondary,
            )))]
        } else {
            items
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Shelve Entries ")
                .border_style(border_style),
        )
        .highlight_style(self.styles.highlight);

        f.render_stateful_widget(list, area, &mut self.shelve_state);
    }

    fn handle_stash_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.stash_entries.len();
                if len > 0 {
                    let i = self.stash_state.selected().unwrap_or(0);
                    self.stash_state.select(Some((i + 1).min(len - 1)));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.stash_state.selected().unwrap_or(0);
                self.stash_state.select(Some(i.saturating_sub(1)));
                None
            }
            KeyCode::Char('G') => {
                if !self.stash_entries.is_empty() {
                    self.stash_state.select(Some(self.stash_entries.len() - 1));
                }
                None
            }
            KeyCode::Char('g') => {
                self.stash_state.select(Some(0));
                None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let len = self.stash_entries.len();
                if len > 0 {
                    let i = self.stash_state.selected().unwrap_or(0);
                    self.stash_state.select(Some((i + 10).min(len - 1)));
                }
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let i = self.stash_state.selected().unwrap_or(0);
                self.stash_state.select(Some(i.saturating_sub(10)));
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.selected_stash_index() {
                    Some(Action::StashPop(idx))
                } else {
                    None
                }
            }
            KeyCode::Char('a') => {
                if let Some(idx) = self.selected_stash_index() {
                    Some(Action::StashApply(idx))
                } else {
                    None
                }
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.selected_stash_index() {
                    Some(Action::StashDrop(idx))
                } else {
                    None
                }
            }
            KeyCode::Char('s') => Some(Action::Stash),
            _ => None,
        }
    }

    fn handle_shelve_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.shelve_entries.len();
                if len > 0 {
                    let i = self.shelve_state.selected().unwrap_or(0);
                    self.shelve_state.select(Some((i + 1).min(len - 1)));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.shelve_state.selected().unwrap_or(0);
                self.shelve_state.select(Some(i.saturating_sub(1)));
                None
            }
            KeyCode::Char('G') => {
                if !self.shelve_entries.is_empty() {
                    self.shelve_state
                        .select(Some(self.shelve_entries.len() - 1));
                }
                None
            }
            KeyCode::Char('g') => {
                self.shelve_state.select(Some(0));
                None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let len = self.shelve_entries.len();
                if len > 0 {
                    let i = self.shelve_state.selected().unwrap_or(0);
                    self.shelve_state.select(Some((i + 10).min(len - 1)));
                }
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let i = self.shelve_state.selected().unwrap_or(0);
                self.shelve_state.select(Some(i.saturating_sub(10)));
                None
            }
            KeyCode::Enter => {
                if let Some(name) = self.selected_shelve_name() {
                    Some(Action::ShelveApply(name))
                } else {
                    None
                }
            }
            KeyCode::Char('d') => {
                if let Some(name) = self.selected_shelve_name() {
                    Some(Action::ShelveDrop(name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
