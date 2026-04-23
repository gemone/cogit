use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::navigation::handle_list_navigation;
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
        // Handle basic navigation with the shared helper
        if handle_list_navigation(&mut self.stash_state, self.stash_entries.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let len = self.stash_entries.len();
                if len > 0 {
                    let i = self.stash_state.selected().unwrap_or(0);
                    self.stash_state.select(Some((i + 10).min(len - 1)));
                }
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let i = self.stash_state.selected().unwrap_or(0);
                self.stash_state.select(Some(i.saturating_sub(10)));
                None
            }
            KeyCode::Enter => self.selected_stash_index().map(Action::StashPop),
            KeyCode::Char('a') => self.selected_stash_index().map(Action::StashApply),
            KeyCode::Char('d') => self.selected_stash_index().map(Action::StashDrop),
            KeyCode::Char('s') => Some(Action::Stash),
            _ => None,
        }
    }

    fn handle_shelve_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle basic navigation with the shared helper
        if handle_list_navigation(&mut self.shelve_state, self.shelve_entries.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let len = self.shelve_entries.len();
                if len > 0 {
                    let i = self.shelve_state.selected().unwrap_or(0);
                    self.shelve_state.select(Some((i + 10).min(len - 1)));
                }
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let i = self.shelve_state.selected().unwrap_or(0);
                self.shelve_state.select(Some(i.saturating_sub(10)));
                None
            }
            KeyCode::Enter => self.selected_shelve_name().map(|name| Action::ShelveApply(
                self.shelve_entries.iter().position(|e| e.name == name).unwrap_or(0),
                false,
            )),
            KeyCode::Char('d') => self.selected_shelve_name().map(|name| Action::ShelveDrop(
                self.shelve_entries.iter().position(|e| e.name == name).unwrap_or(0),
            )),
            _ => None,
        }
    }
}
