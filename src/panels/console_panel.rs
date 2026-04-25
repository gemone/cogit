use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::any::Any;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use super::{Action, Panel};
use crate::app::navigation::handle_list_navigation;
use crate::app::styles::Styles;
use crate::gitops::types::OperationEntry;

const MAX_ENTRIES: usize = 500;

pub struct ConsolePanel {
    focused: bool,
    state: ListState,
    entries: Vec<OperationEntry>,
    log_path: PathBuf,
    styles: Styles,
}

impl ConsolePanel {
    pub fn new(_repo: &Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(None);

        let data_dir = directories::ProjectDirs::from("one", "gemo", "cogit")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir());

        let log_path = data_dir.join("operations.log");

        let mut panel = Self {
            focused: false,
            state,
            entries: Vec::new(),
            log_path,
            styles: styles.clone(),
        };
        panel.load_from_disk();
        // Select last entry (most recent)
        if !panel.entries.is_empty() {
            panel.state.select(Some(panel.entries.len() - 1));
        }
        panel
    }

    /// Load existing entries from disk
    fn load_from_disk(&mut self) {
        if !self.log_path.exists() {
            return;
        }
        if let Ok(file) = std::fs::File::open(&self.log_path) {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Ok(entry) = serde_json::from_str::<OperationEntry>(&line) {
                    self.entries.push(entry);
                }
            }
            // Keep only last MAX_ENTRIES in memory
            if self.entries.len() > MAX_ENTRIES {
                let drain_count = self.entries.len() - MAX_ENTRIES;
                self.entries.drain(0..drain_count);
            }
        }
    }

    /// Append a new operation entry (in-memory + disk)
    pub fn record(&mut self, action: &str, detail: &str, result: &str) {
        let entry = OperationEntry::new(action, detail, result);
        if let Some(parent) = self.log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            if let Ok(json) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{}", json);
            }
        }
        self.entries.push(entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.state.select(Some(self.entries.len().saturating_sub(1)));
    }
}

impl Panel for ConsolePanel {
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

        let title = format!(" Console [{}] ", self.entries.len());

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| {
                let result_style = if e.is_ok() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };

                let action_style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);

                let line = Line::from(vec![
                    Span::styled(format!("{} ", e.timestamp), self.styles.text_secondary),
                    Span::styled(format!("{:<12}", e.action), action_style),
                    Span::styled(
                        if e.detail.chars().count() > 40 {
                            let truncated: String = e.detail.chars().take(39).collect();
                            format!("{}… ", truncated)
                        } else {
                            format!("{:<41}", e.detail)
                        },
                        self.styles.text_primary,
                    ),
                    Span::styled(
                        if e.is_ok() { "✓".to_string() } else { "✗ ".to_string() },
                        result_style,
                    ),
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
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if handle_list_navigation(&mut self.state, self.entries.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('G') => {
                // Jump to bottom (latest)
                if !self.entries.is_empty() {
                    self.state.select(Some(self.entries.len() - 1));
                }
                None
            }
            KeyCode::Char('g') => {
                // Jump to top (oldest)
                self.state.select(Some(0));
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Console"
    }

    fn refresh(&mut self) {
        // No-op: entries are added via record(), not by polling git
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
