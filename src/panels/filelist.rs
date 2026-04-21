use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo, WorktreeFile};
use crate::vimkeys::parse_key_event;

use super::{Action, Mode, Panel};

pub struct FileListPanel {
    focused: bool,
    files: Vec<WorktreeFile>,
    cursor: usize,
}

impl FileListPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            files: Vec::new(),
            cursor: 0,
        }
    }
}

impl Panel for FileListPanel {
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
            .title("Files");

        let items: Vec<ListItem> = if self.files.is_empty() {
            vec![ListItem::new("(no changes)")]
        } else {
            self.files
                .iter()
                .enumerate()
                .map(|(i, file)| {
                    let style = if i == self.cursor && self.focused {
                        styles.selection
                    } else {
                        Style::default()
                    };
                    let prefix = match file.status {
                        crate::gitops::FileStatus::Untracked => "? ",
                        crate::gitops::FileStatus::Modified => "M ",
                        crate::gitops::FileStatus::StagedNew => "A ",
                        crate::gitops::FileStatus::StagedModified => "M ",
                        crate::gitops::FileStatus::Conflicted => "! ",
                        crate::gitops::FileStatus::Ignored => "I ",
                    };
                    let text = format!("{}{}", prefix, file.path);
                    ListItem::new(text).style(style)
                })
                .collect()
        };

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
                        if self.cursor + 1 < self.files.len() {
                            self.cursor += 1;
                        }
                    }
                }
                _ => {}
            }
            return Some(Action::None);
        }

        match key.code {
            KeyCode::Tab => return Some(Action::FocusDiff),
            KeyCode::BackTab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Some(Action::FocusSidebar)
            }
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "filelist"
    }

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        self.files = repo.status()?;
        if self.cursor >= self.files.len() && !self.files.is_empty() {
            self.cursor = self.files.len() - 1;
        }
        Ok(())
    }
}
