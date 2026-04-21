use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo, StashEntry};
use crate::vimkeys::parse_key_event;

use super::{Action, Mode, Panel};

#[derive(Debug, Clone)]
enum SectionKind {
    Head,
    Branches,
    Remotes,
    Tags,
    Stashes,
}

struct SidebarSection {
    kind: SectionKind,
    title: String,
    items: Vec<String>,
    expanded: bool,
}

pub struct SidebarPanel {
    focused: bool,
    sections: Vec<SidebarSection>,
    cursor: usize,
    /// Total flattened items count (section headers + expanded items)
    total_rows: usize,
}

impl SidebarPanel {
    pub fn new() -> Self {
        Self {
            focused: false,
            sections: Vec::new(),
            cursor: 0,
            total_rows: 0,
        }
    }

    fn rebuild_rows(&mut self) {
        self.total_rows = 0;
        for sec in &self.sections {
            self.total_rows += 1; // section header
            if sec.expanded {
                self.total_rows += sec.items.len();
            }
        }
        if self.cursor >= self.total_rows && self.total_rows > 0 {
            self.cursor = self.total_rows - 1;
        }
    }

    fn row_to_section_and_item(&self, row: usize) -> Option<(usize, Option<usize>)> {
        let mut current = 0;
        for (si, sec) in self.sections.iter().enumerate() {
            if row == current {
                return Some((si, None)); // header row
            }
            current += 1;
            if sec.expanded {
                if row < current + sec.items.len() {
                    return Some((si, Some(row - current)));
                }
                current += sec.items.len();
            }
        }
        None
    }

    fn toggle_section(&mut self, section_idx: usize) {
        if let Some(sec) = self.sections.get_mut(section_idx) {
            sec.expanded = !sec.expanded;
            self.rebuild_rows();
        }
    }

    fn get_selected_item(&self) -> Option<(SectionKind, String)> {
        let (si, item_idx) = self.row_to_section_and_item(self.cursor)?;
        let sec = self.sections.get(si)?;
        match item_idx {
            Some(ii) => {
                let name = sec.items.get(ii)?.clone();
                Some((sec.kind.clone(), name))
            }
            None => None,
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

        let header_style = Style::default()
            .fg(ratatui::style::Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let mut items: Vec<ListItem> = Vec::new();
        let mut row = 0;

        for sec in &self.sections {
            let is_cursor = row == self.cursor && self.focused;
            let icon = if sec.expanded { "▾" } else { "▸" };
            let count = sec.items.len();
            let header_text = format!(" {} {} ({})", icon, sec.title, count);
            let style = if is_cursor {
                styles.selection
            } else {
                header_style
            };
            items.push(ListItem::new(Line::from(Span::styled(header_text, style))));
            row += 1;

            if sec.expanded {
                for item in &sec.items {
                    let is_cursor = row == self.cursor && self.focused;
                    let style = if is_cursor {
                        styles.selection
                    } else {
                        Style::default()
                    };
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("   {}", item),
                        style,
                    ))));
                    row += 1;
                }
            }
        }

        let list = List::new(items).block(block);
        list.render(area, buf);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(motion) = parse_key_event(key, Mode::Normal) {
            match motion {
                crate::vimkeys::Motion::Up(n) => {
                    self.cursor = self.cursor.saturating_sub(n);
                }
                crate::vimkeys::Motion::Down(n) => {
                    if self.total_rows > 0 {
                        self.cursor = (self.cursor + n).min(self.total_rows - 1);
                    }
                }
                crate::vimkeys::Motion::Top => {
                    self.cursor = 0;
                }
                crate::vimkeys::Motion::Bottom => {
                    if self.total_rows > 0 {
                        self.cursor = self.total_rows - 1;
                    }
                }
                _ => {}
            }
            return Some(Action::None);
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some((si, item_idx)) = self.row_to_section_and_item(self.cursor) {
                    match item_idx {
                        None => {
                            // Clicked section header — toggle
                            self.toggle_section(si);
                        }
                        Some(_ii) => {
                            // Clicked an item — if it's a branch, checkout
                            if let Some((SectionKind::Branches, name)) = self.get_selected_item()
                            {
                                return Some(Action::CheckoutBranch(name));
                            }
                        }
                    }
                }
                Some(Action::None)
            }
            KeyCode::Char('c') => {
                if let Some((SectionKind::Branches, name)) = self.get_selected_item() {
                    return Some(Action::CheckoutBranch(name));
                }
                None
            }
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

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        let mut sections = Vec::new();

        // HEAD section
        let head_info = match repo.head_shorthand() {
            Some(branch) => {
                let log = repo.log_oneline(1).unwrap_or_default();
                let last_commit = log.lines().next().unwrap_or("(no commits)");
                format!("{}: {}", branch, last_commit)
            }
            None => "(detached HEAD)".to_string(),
        };
        sections.push(SidebarSection {
            kind: SectionKind::Head,
            title: "HEAD".to_string(),
            items: vec![head_info],
            expanded: true,
        });

        // Branches
        let branches = repo.branches().unwrap_or_default();
        let branch_names: Vec<String> = branches
            .iter()
            .filter(|b| !b.is_remote)
            .map(|b| {
                if let Some(ref up) = b.upstream {
                    format!("{} → {}", b.name, up)
                } else {
                    b.name.clone()
                }
            })
            .collect();
        sections.push(SidebarSection {
            kind: SectionKind::Branches,
            title: "Branches".to_string(),
            items: branch_names,
            expanded: false,
        });

        // Remotes
        let remote_names: Vec<String> = branches
            .iter()
            .filter(|b| b.is_remote)
            .map(|b| b.name.clone())
            .collect();
        sections.push(SidebarSection {
            kind: SectionKind::Remotes,
            title: "Remotes".to_string(),
            items: remote_names,
            expanded: false,
        });

        // Tags
        let tags = repo.list_tags().unwrap_or_default();
        sections.push(SidebarSection {
            kind: SectionKind::Tags,
            title: "Tags".to_string(),
            items: tags,
            expanded: false,
        });

        // Stashes
        let stashes: Vec<String> = repo
            .stash_list()
            .unwrap_or_default()
            .iter()
            .map(|s: &StashEntry| format!("#{}: {}", s.index, s.message))
            .collect();
        sections.push(SidebarSection {
            kind: SectionKind::Stashes,
            title: "Stashes".to_string(),
            items: stashes,
            expanded: false,
        });

        self.sections = sections;
        self.rebuild_rows();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
