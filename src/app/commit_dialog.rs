use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::styles::Styles;

pub struct CommitDialog {
    pub input: String,
    pub cursor: usize,
    pub visible: bool,
}

impl CommitDialog {
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
            .title(" Commit Message ");

        let inner = block.inner(area);

        // Show prompt
        let prompt = Line::from(vec![
            Span::styled("Enter commit message, ", styles.cmdbar),
            Span::styled("Enter", styles.header),
            Span::styled(" to submit, ", styles.cmdbar),
            Span::styled("Esc", styles.header),
            Span::styled(" to cancel", styles.cmdbar),
        ]);

        let input_line = if self.input.is_empty() {
            Line::from(Span::styled("(type your message)", styles.context))
        } else {
            Line::from(Span::raw(&self.input))
        };

        let text = ratatui::text::Text::from(vec![prompt, Line::raw(""), input_line]);
        let paragraph = Paragraph::new(text);
        block.render(area, buf);
        paragraph.render(inner, buf);
    }
}
