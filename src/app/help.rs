use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::styles::Styles;
use crate::{
    app::{
        keymap::{KeyBindingHint, KeyContext, KeymapManager},
        View,
    },
    vimkeys::Mode,
};

pub struct HelpOverlay {
    visible: bool,
    scroll: u16,
    styles: Styles,
}

impl HelpOverlay {
    pub fn new(styles: &Styles) -> Self {
        Self {
            visible: false,
            scroll: 0,
            styles: styles.clone(),
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.scroll = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => self.close(),
            KeyCode::Char('j') | KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Char('G') => self.scroll = u16::MAX,
            KeyCode::Char('g') => self.scroll = 0,
            _ => {}
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, keymap: &KeymapManager, view: &View, mode: &Mode) {
        if !self.visible {
            return;
        }

        let popup_w = (area.width.saturating_mul(4) / 5).max(60);
        let popup_h = (area.height.saturating_mul(4) / 5).max(18);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear, popup_area);

        let border = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Which Key — {} ", keymap.preset_name()))
            .border_style(self.styles.border_active);
        f.render_widget(border, popup_area);

        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            format!("Preset: {}", keymap.preset_name()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Close: Esc / q / ?    Scroll: j/k, PgUp/PgDn, G/g",
            self.styles.text_secondary,
        )]));
        lines.push(Line::from(""));

        push_section(&mut lines, "Global", keymap.bindings_for(KeyContext::Global));
        push_section(&mut lines, section_title(&view), keymap.bindings_for(section_context(&view)));

        if *mode == Mode::Command {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Command mode",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "  :keymap vim | :keymap helix",
                self.styles.text_primary,
            )]));
        }

        let paragraph = Paragraph::new(lines)
            .style(self.styles.text_primary)
            .scroll((self.scroll, 0));
        f.render_widget(paragraph, inner);
    }
}

fn push_section(lines: &mut Vec<Line<'static>>, title: &str, hints: Vec<KeyBindingHint>) {
    lines.push(Line::from(vec![Span::styled(
        title.to_string(),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    for hint in hints {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<12}", hint.key),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(hint.description, Style::default().fg(Color::White)),
        ]));
    }
}

fn section_title(view: &View) -> &'static str {
    match view {
        View::Main => "Main / Local Changes",
        View::Branches => "Branches",
        View::Log => "Log",
        View::Stash => "Stash",
        View::Remote => "Remote",
        View::Shelve => "Shelve",
        View::Rebase => "Rebase",
    }
}

fn section_context(view: &View) -> KeyContext {
    match view {
        View::Main => KeyContext::Main,
        View::Branches => KeyContext::Branches,
        View::Log => KeyContext::Log,
        View::Rebase => KeyContext::Rebase,
        View::Stash => KeyContext::Stash,
        View::Remote => KeyContext::Remote,
        View::Shelve => KeyContext::Shelve,
    }
}
