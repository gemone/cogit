use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::styles::Styles;

/// Represents a pending dangerous action awaiting user confirmation.
/// Each variant holds the data needed to construct and dispatch the real Action.
#[derive(Debug, Clone)]
pub enum Confirmation {
    DeleteBranch(String),
    DropStash(usize),
    ApplyStash(usize),
    PopStash(usize),
    RemoveWorktree(String),
    DeleteTag(String),
    RemoveRemote(String),
    ForceCheckout(String),
}

impl Confirmation {
    pub fn title(&self) -> &'static str {
        match self {
            Confirmation::DeleteBranch(_) => "Delete Branch",
            Confirmation::DropStash(_) => "Drop Stash",
            Confirmation::ApplyStash(_) => "Apply Stash",
            Confirmation::PopStash(_) => "Pop Stash",
            Confirmation::RemoveWorktree(_) => "Remove Worktree",
            Confirmation::DeleteTag(_) => "Delete Tag",
            Confirmation::RemoveRemote(_) => "Remove Remote",
            Confirmation::ForceCheckout(_) => "Force Checkout",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Confirmation::DeleteBranch(name) => {
                format!("Delete branch '{}'? This cannot be undone.", name)
            }
            Confirmation::DropStash(idx) => format!("Drop stash #{}? This cannot be undone.", idx),
            Confirmation::ApplyStash(idx) => format!("Apply stash #{}?", idx),
            Confirmation::PopStash(idx) => format!("Pop stash #{}?", idx),
            Confirmation::RemoveWorktree(path) => {
                format!("Remove worktree at '{}'? Files will be kept on disk.", path)
            }
            Confirmation::DeleteTag(name) => {
                format!("Delete tag '{}'? This cannot be undone.", name)
            }
            Confirmation::RemoveRemote(name) => {
                format!("Remove remote '{}'? This cannot be undone.", name)
            }
            Confirmation::ForceCheckout(name) => {
                format!(
                    "Force checkout to '{}'? Local changes will be discarded.",
                    name
                )
            }
        }
    }

    /// Returns the action to dispatch on confirmation (index 0 = confirm)
    pub fn to_action(&self) -> crate::panels::Action {
        match self {
            Confirmation::DeleteBranch(name) => crate::panels::Action::DeleteBranch(name.clone()),
            Confirmation::DropStash(idx) => crate::panels::Action::StashDrop(*idx),
            Confirmation::ApplyStash(idx) => crate::panels::Action::StashApply(*idx),
            Confirmation::PopStash(idx) => crate::panels::Action::StashPop(*idx),
            Confirmation::RemoveWorktree(path) => crate::panels::Action::RemoveWorktree(path.clone()),
            Confirmation::DeleteTag(name) => crate::panels::Action::DeleteTag(name.clone()),
            Confirmation::RemoveRemote(name) => crate::panels::Action::RemoveRemote(name.clone()),
            Confirmation::ForceCheckout(name) => crate::panels::Action::ForceCheckout(name.clone()),
        }
    }

    pub fn from_action(action: &crate::panels::Action) -> Option<Self> {
        match action {
            crate::panels::Action::DeleteBranch(name) => {
                Some(Confirmation::DeleteBranch(name.clone()))
            }
            crate::panels::Action::StashDrop(idx) => Some(Confirmation::DropStash(*idx)),
            crate::panels::Action::StashPop(idx) => Some(Confirmation::PopStash(*idx)),
            crate::panels::Action::StashApply(idx) => Some(Confirmation::ApplyStash(*idx)),
            crate::panels::Action::RemoveWorktree(path) => {
                Some(Confirmation::RemoveWorktree(path.clone()))
            }
            crate::panels::Action::DeleteTag(name) => Some(Confirmation::DeleteTag(name.clone())),
            crate::panels::Action::RemoveRemote(name) => {
                Some(Confirmation::RemoveRemote(name.clone()))
            }
            crate::panels::Action::ForceCheckout(name) => {
                Some(Confirmation::ForceCheckout(name.clone()))
            }
            _ => None,
        }
    }
}

/// The interactive confirmation dialog popup.
#[derive(Debug, Clone)]
pub struct ConfirmationDialog {
    pub confirmation: Confirmation,
    pub selected: usize,
}

impl ConfirmationDialog {
    pub fn new(confirmation: Confirmation) -> Self {
        Self {
            confirmation,
            selected: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        // 0=Confirm, 1=Cancel
        if self.selected < 1 {
            self.selected += 1;
        }
    }

    /// Returns true if user confirmed (index 0), false if cancelled (index 1)
    pub fn confirmed(&self) -> bool {
        self.selected == 0
    }

    pub fn render(&self, f: &mut Frame, area: Rect, styles: &Styles) {
        let width = 60.min(area.width.saturating_sub(4)).max(40).min(area.width);
        let height = 9.min(area.height);
        let x = area.width.saturating_sub(width) / 2;
        let y = area.height.saturating_sub(height) / 2;

        let popup_area = Rect::new(x, y, width, height);

        // Background
        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(
            Block::default()
                .style(Style::default().bg(Color::Black).fg(Color::White))
                .borders(Borders::ALL)
                .border_style(styles.border_active),
            popup_area,
        );

        // Title
        let inner_w = width.saturating_sub(2).max(1);
        let title_area = Rect::new(x + 1, y + 1, inner_w, 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {} ", self.confirmation.title()),
                styles.text_primary.add_modifier(Modifier::BOLD),
            ))),
            title_area,
        );

        // Message
        let msg_area = Rect::new(x + 1, y + 3, inner_w, 2);
        let msg_lines: Vec<Line> = self
            .confirmation
            .message()
            .chars()
            .collect::<Vec<_>>()
            .chunks(inner_w.max(1) as usize)
            .take(2)
            .map(|chunk| Line::from(Span::raw(chunk.iter().collect::<String>())))
            .collect();
        f.render_widget(
            Paragraph::new(msg_lines).style(styles.addition),
            msg_area,
        );

        // Confirm + Cancel options
        let inner_x = x + 2;
        let inner_w_opt = width.saturating_sub(4).max(1);
        self.render_option(f, inner_x, y + 5, inner_w_opt, "y", "Confirm", 0, styles);
        self.render_option(f, inner_x, y + 6, inner_w_opt, "n", "Cancel", 1, styles);

        // Footer hint
        let hint_area = Rect::new(x + 1, y + height.saturating_sub(1), inner_w, 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " j/k: select  y/n: quick  Enter: confirm  Esc: cancel ",
                styles.text_secondary,
            ))),
            hint_area,
        );
    }

    fn render_option(
        &self,
        f: &mut Frame,
        x: u16,
        y: u16,
        width: u16,
        key: &str,
        label: &str,
        index: usize,
        styles: &Styles,
    ) {
        let area = Rect::new(x, y, width, 1);
        let is_selected = self.selected == index;
        let prefix = if is_selected { "> " } else { "  " };
        let key_style = if is_selected {
            styles.highlight
        } else {
            styles.text_primary.fg(Color::LightYellow)
        };
        let label_style = if is_selected {
            styles.text_primary.add_modifier(Modifier::BOLD)
        } else {
            styles.text_secondary
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("[{}] ", key), key_style),
                Span::styled(label.to_string(), label_style),
            ])),
            area,
        );
    }
}
