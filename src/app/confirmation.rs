use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

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
        }
    }

    pub fn confirm_label(&self) -> &'static str {
        "Confirm"
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
        }
    }

    /// Build a Confirmation from an Action (for dangerous actions intercepted at dispatch).
    /// Only called for actions that are in the dispatch interception list.
    pub fn from_action(action: &crate::panels::Action) -> Self {
        match action {
            crate::panels::Action::DeleteBranch(name) => {
                Confirmation::DeleteBranch(name.clone())
            }
            crate::panels::Action::StashDrop(idx) => Confirmation::DropStash(*idx),
            crate::panels::Action::StashPop(idx) => Confirmation::PopStash(*idx),
            crate::panels::Action::StashApply(idx) => Confirmation::ApplyStash(*idx),
            crate::panels::Action::RemoveWorktree(path) => {
                Confirmation::RemoveWorktree(path.clone())
            }
            crate::panels::Action::DeleteTag(name) => Confirmation::DeleteTag(name.clone()),
            crate::panels::Action::RemoveRemote(name) => Confirmation::RemoveRemote(name.clone()),
            _ => unreachable!(
                "Confirmation::from_action called with an action that does not require confirmation"
            ),
        }
    }
}
#[derive(Debug, Clone)]
pub struct ConfirmOption {
    pub label: String,
    pub action_label: String,
}

impl ConfirmOption {
    pub fn yes() -> Self {
        Self {
            label: "Yes".to_string(),
            action_label: "y".to_string(),
        }
    }

    pub fn no() -> Self {
        Self {
            label: "No".to_string(),
            action_label: "n".to_string(),
        }
    }

    pub fn cancel() -> Self {
        Self {
            label: "Cancel".to_string(),
            action_label: "c".to_string(),
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

    pub fn render(&self, f: &mut Frame, area: Rect) {
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
                .border_style(Style::default().fg(Color::White)),
            popup_area,
        );

        // Title
        let inner_w = width.saturating_sub(2).max(1);
        let title_area = Rect::new(x + 1, y + 1, inner_w, 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {} ", self.confirmation.title()),
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
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
            Paragraph::new(msg_lines).style(Style::default().fg(Color::LightGreen)),
            msg_area,
        );

        // Confirm option
        let confirm_area = Rect::new(x + 2, y + 5, width.saturating_sub(4).max(1), 1);
        let confirm_is_selected = self.selected == 0;
        let confirm_prefix = if confirm_is_selected { "> " } else { "  " };
        let confirm_key_style = if confirm_is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::LightYellow)
        };
        let confirm_label_style = if confirm_is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(confirm_prefix),
                Span::styled(format!("[y] "), confirm_key_style),
                Span::styled(self.confirmation.confirm_label(), confirm_label_style),
            ])),
            confirm_area,
        );

        // Cancel option
        let cancel_area = Rect::new(x + 2, y + 6, width.saturating_sub(4).max(1), 1);
        let cancel_is_selected = self.selected == 1;
        let cancel_prefix = if cancel_is_selected { "> " } else { "  " };
        let cancel_key_style = if cancel_is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::LightYellow)
        };
        let cancel_label_style = if cancel_is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(cancel_prefix),
                Span::styled("[n] ", cancel_key_style),
                Span::styled("Cancel", cancel_label_style),
            ])),
            cancel_area,
        );
    }
}
