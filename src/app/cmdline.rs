use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::styles::Styles;

pub struct Cmdline {
    pub input: String,
    pub cursor: usize,
    pub visible: bool,
}

impl Cmdline {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            visible: false,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
        self.cursor = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
        self.cursor = 0;
    }

    pub fn push_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor += 1;
        }
    }

    pub fn submit(&mut self) -> String {
        let text = self.input.clone();
        self.close();
        text
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        if !self.visible {
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(styles.border_active)
            .title("Command");
        let text = ratatui::text::Text::raw(&self.input);
        let paragraph = Paragraph::new(text).block(block);
        paragraph.render(area, buf);
    }
}
