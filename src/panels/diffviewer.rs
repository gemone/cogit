use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo};
use crate::vimkeys::parse_key_event;

use super::{Action, Mode, Panel};

pub struct DiffViewerPanel {
    focused: bool,
    content: Vec<String>,
    cursor: usize,
}

impl DiffViewerPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            content: vec!["(no diff selected)".to_string()],
            cursor: 0,
        }
    }
}

impl Panel for DiffViewerPanel {
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
            .title("Diff");

        let text = self.content.join("\n");
        let paragraph = Paragraph::new(text).block(block);
        paragraph.render(area, buf);
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
                        if self.cursor + 1 < self.content.len() {
                            self.cursor += 1;
                        }
                    }
                }
                crate::vimkeys::Motion::PageUp => {
                    self.cursor = self.cursor.saturating_sub(10);
                }
                crate::vimkeys::Motion::PageDown => {
                    self.cursor = (self.cursor + 10).min(self.content.len().saturating_sub(1));
                }
                _ => {}
            }
            return Some(Action::None);
        }

        match key.code {
            KeyCode::Tab => return Some(Action::FocusSidebar),
            KeyCode::BackTab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Some(Action::FocusFilelist)
            }
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "diff"
    }

    fn refresh(&mut self, _repo: &mut Repo) -> Result<(), GitError> {
        // TODO: populate real diff in P3
        Ok(())
    }
}
