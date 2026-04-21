use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo};
use crate::vimkeys::parse_key_event;

use super::{Action, Mode, Panel};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffLineKind {
    Header,
    Addition,
    Deletion,
    HunkHeader,
    Context,
}

fn classify_diff_line(line: &str) -> DiffLineKind {
    if line.starts_with("diff ") || line.starts_with("index ") || line.starts_with("---") || line.starts_with("+++") {
        DiffLineKind::Header
    } else if line.starts_with("@@") {
        DiffLineKind::HunkHeader
    } else if line.starts_with('+') {
        DiffLineKind::Addition
    } else if line.starts_with('-') {
        DiffLineKind::Deletion
    } else {
        DiffLineKind::Context
    }
}

pub struct DiffViewerPanel {
    focused: bool,
    content: Vec<String>,
    kinds: Vec<DiffLineKind>,
    offset: usize,
    current_file: Option<String>,
}

impl DiffViewerPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            content: vec!["(no diff selected)".to_string()],
            kinds: vec![DiffLineKind::Context],
            offset: 0,
            current_file: None,
        }
    }

    pub fn load_diff(&mut self, diff_text: String, path: &str) {
        self.current_file = Some(path.to_string());
        if diff_text.is_empty() {
            self.content = vec![format!("(no diff for {})", path)];
            self.kinds = vec![DiffLineKind::Context];
        } else {
            self.content = diff_text.lines().map(|l| l.to_string()).collect();
            self.kinds = self.content.iter().map(|l| classify_diff_line(l)).collect();
        }
        self.offset = 0;
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
        let title = match &self.current_file {
            Some(p) => format!("Diff: {}", p),
            None => "Diff".to_string(),
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title);

        let inner = block.inner(area);
        let visible_height = inner.height as usize;

        let end = (self.offset + visible_height).min(self.content.len());
        let lines: Vec<Line> = self.content[self.offset..end]
            .iter()
            .zip(&self.kinds[self.offset..end])
            .map(|(line, kind)| {
                let style = match kind {
                    DiffLineKind::Addition => styles.addition,
                    DiffLineKind::Deletion => styles.deletion,
                    DiffLineKind::HunkHeader => styles.header,
                    DiffLineKind::Header => styles.header,
                    DiffLineKind::Context => styles.context,
                };
                Line::from(Span::styled(line.as_str(), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        block.render(area, buf);
        paragraph.render(inner, buf);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let visible_height = 20usize; // approximate; render will clamp

        if let Some(motion) = parse_key_event(key, Mode::Normal) {
            match motion {
                crate::vimkeys::Motion::Up(n) => {
                    self.offset = self.offset.saturating_sub(n);
                }
                crate::vimkeys::Motion::Down(n) => {
                    self.offset = (self.offset + n).min(self.content.len().saturating_sub(1));
                }
                crate::vimkeys::Motion::Top => {
                    self.offset = 0;
                }
                crate::vimkeys::Motion::Bottom => {
                    self.offset = self.content.len().saturating_sub(visible_height);
                }
                crate::vimkeys::Motion::PageUp => {
                    self.offset = self.offset.saturating_sub(visible_height);
                }
                crate::vimkeys::Motion::PageDown => {
                    self.offset = (self.offset + visible_height).min(self.content.len().saturating_sub(1));
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
            KeyCode::Char('s') => {
                if self.current_file.is_some() {
                    return Some(Action::Stage);
                }
                None
            }
            KeyCode::Char('u') => {
                if self.current_file.is_some() {
                    return Some(Action::Unstage);
                }
                None
            }
            _ => None,
        }
    }

    fn title(&self) -> &str {
        "diff"
    }

    fn refresh(&mut self, _repo: &mut Repo) -> Result<(), GitError> {
        // Diff content is loaded on demand via load_diff
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
