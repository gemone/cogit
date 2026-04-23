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

pub struct RemotePanel {
    repo: std::path::PathBuf,
    focused: bool,
    state: ListState,
    remotes: Vec<crate::gitops::types::RemoteInfo>,
    styles: Styles,
    input_mode: bool,
    input_buffer: String,
    input_prompt: String,
    input_callback: Option<fn(String, String) -> Action>,
    input_arg: String,
}

impl RemotePanel {
    pub fn new(repo: &std::path::Path, styles: &Styles) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        let mut panel = Self {
            repo: repo.to_path_buf(),
            focused: false,
            state,
            remotes: Vec::new(),
            styles: styles.clone(),
            input_mode: false,
            input_buffer: String::new(),
            input_prompt: String::new(),
            input_callback: None,
            input_arg: String::new(),
        };
        panel.refresh();
        panel
    }

    fn selected_remote_name(&self) -> Option<String> {
        let i = self.state.selected().unwrap_or(0);
        self.remotes.get(i).map(|r| r.name.clone())
    }

    fn selected_remote_url(&self) -> Option<String> {
        let i = self.state.selected().unwrap_or(0);
        self.remotes.get(i).map(|r| r.url.clone())
    }
}

impl Panel for RemotePanel {
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

        let title = if self.input_mode {
            format!(" Remotes [{}: {}] ", self.input_prompt, self.input_buffer)
        } else {
            " Remotes ".to_string()
        };

        let items: Vec<ListItem> = self
            .remotes
            .iter()
            .map(|r| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", r.name),
                        self.styles.addition.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&r.url, self.styles.text_secondary),
                ]);
                ListItem::new(line)
            })
            .collect();

        let empty_msg = if self.remotes.is_empty() {
            "No remotes configured"
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
            "a:add d:delete r:rename u:fetch Enter:branches q:back"
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
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input_buffer.clear();
                    self.input_callback = None;
                    self.input_arg.clear();
                    return None;
                }
                KeyCode::Enter => {
                    let buf = std::mem::take(&mut self.input_buffer);
                    let arg = std::mem::take(&mut self.input_arg);
                    let cb = self.input_callback.take();
                    self.input_mode = false;
                    if let Some(callback) = cb {
                        return Some(callback(arg, buf));
                    }
                    return None;
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

        if handle_list_navigation(&mut self.state, self.remotes.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('a') => {
                self.input_mode = true;
                self.input_prompt = "name".to_string();
                self.input_buffer.clear();
                self.input_callback = Some(Action::AddRemote);
                self.input_arg.clear();
                // Two-step input: first name, then URL
                // For simplicity, we'll use a format like "name url" in one line
                None
            }
            KeyCode::Char('d') => self.selected_remote_name().map(Action::RemoveRemote),
            KeyCode::Char('r') => {
                if let Some(name) = self.selected_remote_name() {
                    self.input_mode = true;
                    self.input_prompt = format!("rename '{}' to", name);
                    self.input_buffer.clear();
                    self.input_callback = Some(Action::RenameRemote);
                    self.input_arg = name;
                }
                None
            }
            KeyCode::Char('u') => self.selected_remote_name().map(Action::FetchRemote),
            KeyCode::Enter => self.selected_remote_name().map(Action::ShowRemoteBranches),
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "Remotes"
    }

    fn refresh(&mut self) {
        if let Ok(repo) = Repository::open(&self.repo) {
            self.remotes = repo.remotes().unwrap_or_default();
        }
        if self.remotes.is_empty() {
            self.state.select(None);
        } else if self.state.selected().unwrap_or(0) >= self.remotes.len() {
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
