use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::styles::Styles;
use crate::gitops::{GitError, Repo};

use super::{Action, Panel};
use crossterm::event::KeyEvent;

pub struct CmdbarPanel {
    title: String,
    branch: String,
    operation: String,
}

impl CmdbarPanel {
    pub fn new() -> Self {
        Self {
            title: "cogit".to_string(),
            branch: String::new(),
            operation: String::new(),
        }
    }
}

impl Panel for CmdbarPanel {
    fn focus(&mut self) {}
    fn blur(&mut self) {}

    fn render(&self, area: Rect, buf: &mut Buffer, styles: &Styles) {
        let mut spans = vec![
            Span::styled(&self.title, styles.header),
            Span::raw(" | "),
            Span::styled(&self.branch, styles.cmdbar_active),
        ];
        if !self.operation.is_empty() {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(&self.operation, styles.conflict));
        }
        spans.push(Span::raw(" | ? help"));
        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    fn title(&self) -> &str {
        "cmdbar"
    }

    fn refresh(&mut self, repo: &mut Repo) -> Result<(), GitError> {
        self.branch = repo
            .head_shorthand()
            .unwrap_or_else(|| "(detached)".to_string());
        // TODO: detect rebase/merge state in P3
        self.operation.clear();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
