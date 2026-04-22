use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub created_at: Instant,
    pub duration: Duration,
    pub is_error: bool,
}

impl Notification {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            is_error: false,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            message: message.to_string(),
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            is_error: true,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }
}

pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn push(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn notify(&mut self, message: &str) {
        self.push(Notification::new(message));
    }

    pub fn notify_error(&mut self, message: &str) {
        self.push(Notification::error(message));
    }

    pub fn cleanup(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.cleanup();
        if self.notifications.is_empty() {
            return;
        }

        // Render notifications at bottom-right
        let max_width = 60.min(area.width as usize);
        let max_visible = 3.min(self.notifications.len());

        for (i, notif) in self.notifications.iter().rev().take(max_visible).enumerate() {
            let style = if notif.is_error {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            };

            let msg = if notif.message.len() > max_width - 2 {
                format!(" {}… ", &notif.message[..max_width - 3])
            } else {
                format!(" {} ", notif.message)
            };

            let width = msg.len() as u16;
            let y = area.bottom().saturating_sub(max_visible as u16 - i as u16);
            let x = area.right().saturating_sub(width);

            let notif_area = Rect {
                x,
                y,
                width,
                height: 1,
            };

            let paragraph = Paragraph::new(Line::from(msg)).style(style);
            f.render_widget(paragraph, notif_area);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}
