use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{FileStatus, GitError, Repo, WorktreeFile};
use crate::panels::{Action, Mode, Panel};
use crate::vimkeys::{parse_key_event, Motion};

pub struct FileListPanel {
    focused: bool,
    files: Vec<WorktreeFile>,
    cursor: usize,
    offset: usize,
    show_ignored: bool,
    show_untracked: bool,
}

impl Default for FileListPanel {
    fn default() -> Self {
        Self {
            focused: false,
            files: Vec::new(),
            cursor: 0,
            offset: 0,
            show_ignored: false,
            show_untracked: true,
        }
    }
}

impl FileListPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_file(&self) -> Option<&WorktreeFile> {
        self.files.get(self.cursor)
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    fn is_staged(&self, file: &WorktreeFile) -> bool {
        matches!(file.status, FileStatus::StagedNew | FileStatus::StagedModified)
    }

    fn status_icon_and_style<'a>(&self, status: &FileStatus, styles: &'a Styles) -> (char, Style) {
        match status {
            FileStatus::Untracked => ('?', styles.status_untracked),
            FileStatus::Modified => ('M', styles.status_modified),
            FileStatus::StagedNew => ('A', styles.status_staged),
            FileStatus::StagedModified => ('M', styles.status_staged),
            FileStatus::Conflicted => ('C', styles.status_conflicted),
            FileStatus::Ignored => ('!', styles.status_ignored),
        }
    }

    fn clamp_offset(&mut self, visible_height: usize) {
        if self.cursor < self.offset {
            self.offset = self.cursor;
        }
        if visible_height > 0 && self.cursor >= self.offset + visible_height {
            self.offset = self.cursor.saturating_sub(visible_height - 1);
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

    fn title(&self) -> &str {
        "filelist"
    }

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        let mut files = repo.status()?;
        if !self.show_ignored {
            files.retain(|f| !matches!(f.status, FileStatus::Ignored));
        }
        if !self.show_untracked {
            files.retain(|f| !matches!(f.status, FileStatus::Untracked));
        }
        files.sort_by_key(|f| match f.status {
            FileStatus::Conflicted => 0,
            FileStatus::Untracked => 1,
            FileStatus::Modified => 1,
            FileStatus::StagedNew => 2,
            FileStatus::StagedModified => 2,
            FileStatus::Ignored => 3,
        });
        self.files = files;
        if self.cursor >= self.files.len() && !self.files.is_empty() {
            self.cursor = self.files.len() - 1;
        }
        if self.files.is_empty() {
            self.cursor = 0;
        }
        self.offset = 0;
        Ok(())
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

        if self.files.is_empty() {
            Paragraph::new("(no changes)").block(block).render(area, buf);
            return;
        }

        // Split files into sections
        let conflicted: Vec<&WorktreeFile> = self
            .files
            .iter()
            .filter(|f| matches!(f.status, FileStatus::Conflicted))
            .collect();
        let changes: Vec<&WorktreeFile> = self
            .files
            .iter()
            .filter(|f| matches!(f.status, FileStatus::Modified | FileStatus::Untracked))
            .collect();
        let staged: Vec<&WorktreeFile> = self
            .files
            .iter()
            .filter(|f| matches!(f.status, FileStatus::StagedNew | FileStatus::StagedModified))
            .collect();

        let mut items: Vec<ListItem> = Vec::new();
        let mut visual_cursor: usize = 0;
        let header_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);

        let sections = [("Conflicted", conflicted), ("Changes", changes), ("Staged", staged)];
        for (title, files) in sections {
            if files.is_empty() {
                continue;
            }
            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {} ({})", title, files.len()),
                header_style,
            ))));
            for file in files {
                let is_selected = self
                    .files
                    .get(self.cursor)
                    .map(|f| f.path == file.path)
                    .unwrap_or(false);
                if is_selected {
                    visual_cursor = items.len();
                }
                let (icon, icon_style) = self.status_icon_and_style(&file.status, styles);
                let item_style = if is_selected && self.focused {
                    styles.selection
                } else {
                    Style::default()
                };
                let line = Line::from(vec![
                    Span::styled(icon.to_string(), icon_style),
                    Span::raw(" "),
                    Span::raw(file.path.clone()),
                ]);
                items.push(ListItem::new(line).style(item_style));
            }
        }

        let inner_area = block.inner(area);
        let visible_height = inner_area.height as usize;

        // Compute render offset based on stored offset and visual cursor
        let mut render_offset = self.offset;
        if visual_cursor < render_offset {
            render_offset = visual_cursor;
        }
        if visible_height > 0 && visual_cursor >= render_offset + visible_height {
            render_offset = visual_cursor.saturating_sub(visible_height - 1);
        }

        let end = (render_offset + visible_height).min(items.len());
        let visible_items = if render_offset < items.len() {
            &items[render_offset..end]
        } else {
            &[]
        };

        List::new(visible_items.to_vec()).block(block).render(area, buf);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(motion) = parse_key_event(key, Mode::Normal) {
            let visible_height = 20usize; // approximate; render will clamp
            match motion {
                Motion::Up(n) => {
                    self.cursor = self.cursor.saturating_sub(n);
                }
                Motion::Down(n) => {
                    if !self.files.is_empty() {
                        self.cursor = (self.cursor + n).min(self.files.len() - 1);
                    }
                }
                Motion::Top => {
                    self.cursor = 0;
                }
                Motion::Bottom => {
                    if !self.files.is_empty() {
                        self.cursor = self.files.len() - 1;
                    }
                }
                Motion::PageUp => {
                    self.cursor = self.cursor.saturating_sub(visible_height);
                }
                Motion::PageDown => {
                    if !self.files.is_empty() {
                        self.cursor = (self.cursor + visible_height).min(self.files.len() - 1);
                    }
                }
                _ => {}
            }
            self.clamp_offset(visible_height);
            return None;
        }

        match key.code {
            KeyCode::Char(' ') => {
                if let Some(file) = self.selected_file() {
                    return if self.is_staged(file) {
                        Some(Action::Unstage)
                    } else {
                        Some(Action::Stage)
                    };
                }
            }
            KeyCode::Char('s') => {
                if self.selected_file().is_some() {
                    return Some(Action::Stage);
                }
            }
            KeyCode::Char('u') => {
                if self.selected_file().is_some() {
                    return Some(Action::Unstage);
                }
            }
            KeyCode::Char('a') => {
                return Some(Action::StageAll);
            }
            KeyCode::Char('A') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Some(Action::UnstageAll);
            }
            KeyCode::Char('d') => {
                if let Some(file) = self.selected_file() {
                    return Some(Action::Discard(file.path.clone()));
                }
            }
            KeyCode::Enter => {
                if let Some(file) = self.selected_file() {
                    return Some(Action::OpenDiff(file.path.clone()));
                }
            }
            KeyCode::Char('i') => {
                if let Some(file) = self.selected_file() {
                    return Some(Action::IgnoreFile(file.path.clone()));
                }
            }
            KeyCode::Char('I') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.show_ignored = !self.show_ignored;
                return Some(Action::Refresh);
            }
            KeyCode::Char('U') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.show_untracked = !self.show_untracked;
                return Some(Action::Refresh);
            }
            KeyCode::Char('/') => {
                return Some(Action::Search);
            }
            KeyCode::Tab => {
                return Some(Action::FocusDiff);
            }
            KeyCode::BackTab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Some(Action::FocusSidebar);
            }
            _ => {}
        }
        None
    }
}
