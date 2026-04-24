use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::any::Any;

use super::{Action, Panel};
use crate::app::navigation::handle_list_navigation;
use crate::gitops::types::{RebaseAction, RebaseTodo};

pub struct RebasePanel {
    focused: bool,
    state: ListState,
    todos: Vec<RebaseTodo>,
    onto: String,
}

impl RebasePanel {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            focused: false,
            state,
            todos: Vec::new(),
            onto: String::new(),
        }
    }

    pub fn load_todos(&mut self, repo: &crate::gitops::Repository, onto: &str) {
        self.onto = onto.to_string();
        self.todos = repo.rebase_get_todo(onto).unwrap_or_default();
        if !self.todos.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    fn cycle_action(&mut self, forward: bool) {
        if let Some(idx) = self.selected_index() {
            let actions = [
                RebaseAction::Pick,
                RebaseAction::ReWord,
                RebaseAction::Edit,
                RebaseAction::Squash,
                RebaseAction::FixUp,
                RebaseAction::Drop,
            ];
            let current = &self.todos[idx].action;
            let pos = actions.iter().position(|a| a == current).unwrap_or(0);
            let new_pos = if forward {
                (pos + 1) % actions.len()
            } else {
                (pos + actions.len() - 1) % actions.len()
            };
            self.todos[idx].action = actions[new_pos].clone();
        }
    }

    fn move_up(&mut self) {
        if let Some(idx) = self.selected_index() {
            if idx > 0 {
                self.todos.swap(idx, idx - 1);
                self.state.select(Some(idx - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(idx) = self.selected_index() {
            if idx < self.todos.len() - 1 {
                self.todos.swap(idx, idx + 1);
                self.state.select(Some(idx + 1));
            }
        }
    }

    fn action_style(action: &RebaseAction) -> Style {
        match action {
            RebaseAction::Pick => Style::default().fg(Color::Green),
            RebaseAction::ReWord => Style::default().fg(Color::Cyan),
            RebaseAction::Edit => Style::default().fg(Color::Yellow),
            RebaseAction::Squash => Style::default().fg(Color::Magenta),
            RebaseAction::FixUp => Style::default().fg(Color::Blue),
            RebaseAction::Drop => Style::default().fg(Color::Red),
        }
    }
}

impl Panel for RebasePanel {
    fn focus(&mut self) { self.focused = true; }
    fn blur(&mut self) { self.focused = false; }
    fn title(&self) -> &str { "Rebase" }

    fn refresh(&mut self) {}

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.focused {
            return None;
        }

        // Check rebase-specific keys BEFORE handle_list_navigation,
        // which intercepts J/K as PageDown/PageUp.
        match key.code {
            KeyCode::Char('J') | KeyCode::Char('N') => {
                self.move_down();
                return None;
            }
            KeyCode::Char('K') | KeyCode::Char('P') => {
                self.move_up();
                return None;
            }
            _ => {}
        }

        if handle_list_navigation(&mut self.state, self.todos.len(), key) {
            return None;
        }

        match key.code {
            KeyCode::Char('s') => {
                self.cycle_action(true);
                None
            }
            KeyCode::Char('S') => {
                self.cycle_action(false);
                None
            }
            KeyCode::Enter => {
                if !self.todos.is_empty() && !self.onto.is_empty() {
                    let todos = self.todos.clone();
                    let onto = self.onto.clone();
                    Some(Action::ExecuteRebase(onto, todos))
                } else {
                    None
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::BackToMain),
            _ => None,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .todos
            .iter()
            .enumerate()
            .map(|(i, todo)| {
                let action_style = Self::action_style(&todo.action);
                let highlight = if self.focused && self.state.selected() == Some(i) {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!(" {} ", todo.action.short()),
                        action_style.add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<8} ", todo.short_hash),
                        highlight.fg(Color::Yellow),
                    ),
                    Span::styled(&todo.subject, highlight),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = if self.onto.is_empty() {
            " Rebase ".to_string()
        } else {
            format!(" Rebase onto {} ", self.onto)
        };

        let border_color = if self.focused { Color::Cyan } else { Color::DarkGray };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.state);
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
