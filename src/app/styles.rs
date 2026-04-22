use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Styles {
    pub border_active: Style,
    pub border_inactive: Style,
    pub text_primary: Style,
    pub text_secondary: Style,
    pub highlight: Style,
    pub addition: Style,
    pub deletion: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            border_active: Style::default().fg(Color::Cyan),
            border_inactive: Style::default().fg(Color::DarkGray),
            text_primary: Style::default().fg(Color::White),
            text_secondary: Style::default().fg(Color::DarkGray),
            highlight: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            addition: Style::default().fg(Color::Green),
            deletion: Style::default().fg(Color::Red),
        }
    }
}
