use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::styles::Styles;

pub struct CmdLine {
    visible: bool,
    input: String,
    cursor: usize,
    styles: Styles,
}

impl CmdLine {
    pub fn new(styles: &Styles) -> Self {
        Self {
            visible: false,
            input: String::new(),
            cursor: 0,
            styles: styles.clone(),
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
        self.input.push(':');
        self.cursor = self.input.len();
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        self.cursor = self.input.len();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 1 {
            self.input.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    pub fn submit(&mut self) -> String {
        let cmd = self.input.trim().to_string();
        self.visible = false;
        self.input.clear();
        cmd
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let display = if self.input.is_empty() {
            ":".to_string()
        } else {
            self.input.clone()
        };

        let cmd_line = Paragraph::new(display)
            .style(self.styles.text_primary.add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(Color::Black)),
            );

        f.render_widget(cmd_line, area);
    }
}
