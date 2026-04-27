use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{
    popup::{
        centered_popup_area, popup_block, popup_inner_area_without_footer, popup_style,
        render_popup_background,
    },
    styles::Styles,
};
use crate::{
    app::{
        View,
        keymap::{KeyBindingHint, KeyContext, KeymapManager},
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

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        keymap: &KeymapManager,
        view: &View,
        mode: &Mode,
    ) {
        if !self.visible {
            return;
        }

        let popup_area = centered_popup_area(area, 4, 5, 60, 4, 5, 18);
        render_popup_background(f, popup_area);

        let title = format!(" Which Key — {} ", keymap.preset_name());
        let border = popup_block(title.as_str(), self.styles.border_active);
        f.render_widget(border, popup_area);

        let inner = popup_inner_area_without_footer(popup_area);
        let paragraph = Paragraph::new(self.content_lines(keymap, view, mode))
            .style(popup_style().fg(self.styles.text_primary.fg.unwrap_or(Color::White)))
            .scroll((self.scroll, 0));
        f.render_widget(paragraph, inner);
    }

    fn content_lines(
        &self,
        keymap: &KeymapManager,
        view: &View,
        mode: &Mode,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            format!("Preset: {}", keymap.preset_name()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Close: Esc / q / ?    Scroll: j/k, PgUp/PgDn, G/g",
            self.styles.text_secondary,
        )]));
        lines.push(Line::from(""));

        push_section(
            &mut lines,
            "Global",
            keymap.bindings_for(KeyContext::Global),
        );
        push_section(
            &mut lines,
            section_title(view),
            keymap.bindings_for(section_context(view)),
        );

        if *mode == Mode::Command {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Command mode",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "  :keymap vim | :keymap helix",
                self.styles.text_primary,
            )]));
        }

        lines
    }

    #[cfg(test)]
    pub(crate) fn debug_lines(
        &self,
        keymap: &KeymapManager,
        view: &View,
        mode: &Mode,
    ) -> Vec<Line<'static>> {
        self.content_lines(keymap, view, mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{View, keymap::KeymapManager, styles::Styles},
        config::CogitConfig,
        vimkeys::Mode,
    };
    use ratatui::{Terminal, backend::TestBackend, widgets::Block};

    #[test]
    fn help_overlay_lists_native_mode_shortcuts_in_global_section() {
        let mut overlay = HelpOverlay::new(&Styles::default());
        let keymap = KeymapManager::new(&CogitConfig::default());
        overlay.open();

        let lines = overlay.debug_lines(&keymap, &View::Main, &Mode::Normal);
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Esc"));
        assert!(rendered.contains("Switch to normal mode"));
        assert!(rendered.contains("i"));
        assert!(rendered.contains("Switch to edit mode"));
        assert!(rendered.contains("v"));
        assert!(rendered.contains("Switch to visual mode"));
    }

    #[test]
    fn help_overlay_clears_background_before_rendering() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut overlay = HelpOverlay::new(&Styles::default());
        let keymap = KeymapManager::new(&CogitConfig::default());
        overlay.open();

        terminal
            .draw(|f| {
                let area = f.area();
                let background = Block::default().style(Style::default().bg(Color::Red));
                f.render_widget(background, area);
                overlay.render(f, area, &keymap, &View::Main, &Mode::Normal);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        assert_eq!(buffer[(12, 8)].bg, Color::Black);
        assert_eq!(buffer[(2, 2)].bg, Color::Red);
    }
}

fn push_section(lines: &mut Vec<Line<'static>>, title: &str, hints: Vec<KeyBindingHint>) {
    lines.push(Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    for hint in hints {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<12}", hint.key),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
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
        View::Console => "Console",
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
        View::Console => KeyContext::Console,
    }
}
