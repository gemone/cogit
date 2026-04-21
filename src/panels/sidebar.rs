use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo};
use crate::vimkeys::parse_key_event;

use super::{Action, Mode, Panel};

pub struct SidebarPanel {
    focused: bool,
    items: Vec<String>,
    cursor: usize,
}

impl SidebarPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            items: vec![
                "HEAD".to_string(),
                "Branches".to_string(),
                "Remotes".to_string(),
                "Tags".to_string(),
                "Stashes".to_string(),
                "Shelves".to_string(),
            ],
            cursor: 0,
        }
    }
}

impl Panel for SidebarPanel {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn blur(&mut self) {
        self.focused = false;
    }

    fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        let border_style = if self.focused {
            styles.border_active
        } else {
            styles.border_inactive
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title("Sidebar");

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let style = if i == self.cursor && self.focused {
                    styles.selection
                } else {
                    Style::default()
                };
                ListItem::new(text.as_str()).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        list.render(area, buf);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(motion) = parse_key_event(key, Mode::Normal) {
            match motion {
                crate::vimkeys::Motion::Up(n) => {
                    for _ in 0..n {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                        }
                    }
                }
                crate::vimkeys::Motion::Down(n) => {
                    for _ in 0..n {
                        if self.cursor + 1 < self.items.len() {
                            self.cursor += 1;
                        }
                    }
                }
                _ => {}
            }
            return Some(Action::None);
        }

        match key.code {
            KeyCode::Tab => return Some(Action::FocusFilelist),
            KeyCode::BackTab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Some(Action::FocusDiff)
            }
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "sidebar"
    }

    fn refresh(&mut self, _repo: &mut Repo) -> Result<(), GitError> {
        // TODO: populate real data in P3
        Ok(())
    }
}
