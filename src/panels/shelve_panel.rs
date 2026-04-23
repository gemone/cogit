use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::navigation::handle_list_navigation;
use crate::app::styles::Styles;
use crate::gitops::Repository;
use crate::gitops::shelve::ShelveEntry;

pub struct ShelvePanel {
    repo: std::path::PathBuf,
    focused: bool,
    state: ListState,
    entries: Vec<ShelveEntry>,
    styles: Styles,
    input_mode: bool,
    input_buffer: String,
    input_prompt: String,
    include_staged: bool,
    diff_popup: Option<(String, u16)>, // (content, scroll)
}

impl ShelvePanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            state,
            entries: Vec::new(),
            styles: styles.clone(),
            input_mode: false,
            input_buffer: String::new(),
            input_prompt: String::new(),
            include_staged: false,
            diff_popup: None,
        };
        panel.refresh();
        panel
    }

    fn selected_index(&self) -> Option<usize> {
        let i = self.state.selected().unwrap_or(0);
        self.entries.get(i).map(|e| e.index)
    }

    fn selected_name(&self) -> Option<String> {
        let i = self.state.selected().unwrap_or(0);
        self.entries.get(i).map(|e| e.name.clone())
    }

    fn selected_shelve_index(&self) -> Option<usize> {
        let i = self.state.selected().unwrap_or(0);
        self.entries.get(i).map(|e| e.index)
    }

    fn close_diff(&mut self) {
        self.diff_popup = None;
    }
}

impl Panel for ShelvePanel {
    fn focus(&mut self) {
        self.focused = true;
        self.refresh();
    }

    fn blur(&mut self) {
        self.focused = false;
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        // If diff popup is open, render it
        if let Some((ref content, scroll)) = self.diff_popup {
            self.render_diff_popup(f, area, content, scroll);
            return;
        }

        let border_style = if self.focused {
            self.styles.border_active
        } else {
            self.styles.border_inactive
        };

        let title = if self.input_mode {
            format!(" Shelves [{}: {}] ", self.input_prompt, self.input_buffer)
        } else {
            let staged_indicator = if self.include_staged { " [+staged]" } else { "" };
            format!(" Shelves{} ", staged_indicator)
        };

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| {
                let staged_marker = if e.has_staged { " [S]" } else { "" };
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}{} ", e.name, staged_marker),
                        self.styles.addition.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&e.date, self.styles.text_secondary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let empty_msg = if self.entries.is_empty() {
            "No shelve entries. Press 'n' to create one."
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
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(self.styles.highlight);

        f.render_stateful_widget(list, area, &mut self.state);

        // Help text at bottom
        let help_text = if self.input_mode {
            "Enter:confirm Esc:cancel"
        } else {
            "n:new s:toggle-staged p:pop a:apply d:drop Enter:diff q:back"
        };
        let help = Paragraph::new(help_text).style(self.styles.text_secondary);
        let help_area = Rect {
            y: area.bottom().saturating_sub(1),
            height: 1,
            ..area
        };
        f.render_widget(help, help_area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle diff popup first
        if self.diff_popup.is_some() {
            if let Some((_, scroll)) = self.diff_popup.as_mut() {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.close_diff();
                        return None;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        *scroll = scroll.saturating_add(1);
                        return None;
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        *scroll = scroll.saturating_sub(1);
                        return None;
                    }
                    KeyCode::PageDown => {
                        *scroll = scroll.saturating_add(10);
                        return None;
                    }
                    KeyCode::PageUp => {
                        *scroll = scroll.saturating_sub(10);
                        return None;
                    }
                    _ => return None,
                }
            }
        }

        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input_buffer.clear();
                    return None;
                }
                KeyCode::Enter => {
                    let name = std::mem::take(&mut self.input_buffer);
                    self.input_mode = false;
                    return Some(Action::ShelveCreate(name, self.include_staged));
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    return None;
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                    return None;
                }
                _ => return None,
            }
        }

        if handle_list_navigation(&mut self.state, self.entries.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('n') => {
                self.input_mode = true;
                self.input_prompt = "name".to_string();
                self.input_buffer.clear();
                None
            }
            KeyCode::Char('s') => {
                self.include_staged = !self.include_staged;
                None
            }
            KeyCode::Char('p') => {
                if let Some(idx) = self.selected_shelve_index() {
                    return Some(Action::ShelveApply(idx, false));
                }
                None
            }
            KeyCode::Char('a') => {
                if let Some(idx) = self.selected_shelve_index() {
                    return Some(Action::ShelveApply(idx, true));
                }
                None
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.selected_shelve_index() {
                    return Some(Action::ShelveDrop(idx));
                }
                None
            }
            KeyCode::Enter => {
                if let Some(index) = self.selected_index() {
                    // Show diff in popup
                    if let Ok(repo) = Repository::open(&self.repo) {
                        if let Ok(content) = repo.shelve_show(index) {
                            self.diff_popup = Some((content, 0));
                        }
                    }
                }
                None
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Shelves"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo) {
            self.entries = repo.list_shelves().unwrap_or_default();
        }
        if self.entries.is_empty() {
            self.state.select(None);
        } else if self.state.selected().unwrap_or(0) >= self.entries.len() {
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

impl ShelvePanel {
    fn render_diff_popup(&self, f: &mut Frame, area: Rect, content: &str, scroll: u16) {
        let popup_w = (area.width * 4 / 5).max(40);
        let popup_h = (area.height * 4 / 5).max(10);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = ratatui::layout::Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(
            ratatui::style::Style::default()
                .bg(ratatui::style::Color::Black)
                .fg(ratatui::style::Color::White),
        );
        f.render_widget(clear, popup_area);

        let inner = ratatui::layout::Rect {
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
                    ratatui::style::Style::default().fg(ratatui::style::Color::Green)
                } else if line.starts_with('-') && !line.starts_with("---") {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Red)
                } else if line.starts_with("@@") {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
                } else {
                    ratatui::style::Style::default().fg(ratatui::style::Color::White)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Shelve Diff (Esc/q:close j/k:scroll) ")
                    .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)),
            )
            .scroll((scroll, 0));

        f.render_widget(paragraph, inner);
    }
}
