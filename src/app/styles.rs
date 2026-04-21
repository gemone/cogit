use ratatui::style::{Color, Modifier, Style};

pub struct Styles {
    pub border_active: Style,
    pub border_inactive: Style,
    pub addition: Style,
    pub deletion: Style,
    pub context: Style,
    pub conflict: Style,
    pub header: Style,
    pub selection: Style,
    pub status_untracked: Style,
    pub status_modified: Style,
    pub status_staged: Style,
    pub status_conflicted: Style,
    pub status_ignored: Style,
    pub cmdbar: Style,
    pub cmdbar_active: Style,
}

impl Styles {
    pub fn dark() -> Self {
        Self {
            border_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            border_inactive: Style::default().fg(Color::DarkGray),
            addition: Style::default().fg(Color::Green),
            deletion: Style::default().fg(Color::Red),
            context: Style::default().fg(Color::Gray),
            conflict: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            selection: Style::default().bg(Color::DarkGray),
            status_untracked: Style::default().fg(Color::Gray),
            status_modified: Style::default().fg(Color::Yellow),
            status_staged: Style::default().fg(Color::Green),
            status_conflicted: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            status_ignored: Style::default().fg(Color::DarkGray),
            cmdbar: Style::default().fg(Color::White),
            cmdbar_active: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn light() -> Self {
        Self {
            border_active: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            border_inactive: Style::default().fg(Color::Gray),
            addition: Style::default().fg(Color::Rgb(0, 128, 0)),
            deletion: Style::default().fg(Color::Rgb(180, 0, 0)),
            context: Style::default().fg(Color::Black),
            conflict: Style::default()
                .fg(Color::Rgb(180, 120, 0))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            selection: Style::default().bg(Color::Rgb(220, 220, 220)),
            status_untracked: Style::default().fg(Color::Gray),
            status_modified: Style::default().fg(Color::Rgb(180, 120, 0)),
            status_staged: Style::default().fg(Color::Rgb(0, 128, 0)),
            status_conflicted: Style::default()
                .fg(Color::Rgb(180, 0, 0))
                .add_modifier(Modifier::BOLD),
            status_ignored: Style::default().fg(Color::Gray),
            cmdbar: Style::default().fg(Color::Black),
            cmdbar_active: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        }
    }
}
