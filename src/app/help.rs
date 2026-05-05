use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{
    Action,
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

/// A display line in the help overlay — either a section header or a binding.
enum HelpLine {
    Header(String),
    Binding(KeyBindingHint),
}

pub struct HelpOverlay {
    visible: bool,
    scroll: u16,
    styles: Styles,
    lines: Vec<HelpLine>,
    selected: usize, // index into actionable binding positions
}

impl HelpOverlay {
    pub fn new(styles: &Styles) -> Self {
        Self {
            visible: false,
            scroll: 0,
            styles: styles.clone(),
            lines: Vec::new(),
            selected: 0,
        }
    }

    pub fn open(&mut self, keymap: &KeymapManager, view: &View, mode: &Mode) {
        self.visible = true;
        self.scroll = 0;
        self.selected = 0;
        self.build_lines(keymap, view, mode);
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Indices of lines that contain actionable bindings.
    fn actionable_indices(&self) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| matches!(l, HelpLine::Binding(b) if b.action.is_some()))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.close();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let indices = self.actionable_indices();
                if let Some(pos) = indices.iter().position(|&i| i == self.selected) {
                    if pos + 1 < indices.len() {
                        self.selected = indices[pos + 1];
                    }
                }
                self.ensure_visible();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let indices = self.actionable_indices();
                if let Some(pos) = indices.iter().position(|&i| i == self.selected) {
                    if pos > 0 {
                        self.selected = indices[pos - 1];
                    }
                }
                self.ensure_visible();
                None
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(10);
                None
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                None
            }
            KeyCode::Char('G') => {
                let indices = self.actionable_indices();
                if let Some(&last) = indices.last() {
                    self.selected = last;
                }
                self.ensure_visible();
                None
            }
            KeyCode::Char('g') => {
                let indices = self.actionable_indices();
                if let Some(&first) = indices.first() {
                    self.selected = first;
                }
                self.scroll = 0;
                None
            }
            KeyCode::Enter => {
                if let HelpLine::Binding(ref b) = self.lines[self.selected] {
                    return b.action.clone();
                }
                None
            }
            _ => None,
        }
    }

    /// Adjust scroll so the selected line is visible.
    fn ensure_visible(&mut self) {
        let sel = self.selected as u16;
        if sel < self.scroll {
            self.scroll = sel;
        } else if sel >= self.scroll + 15 {
            self.scroll = sel.saturating_sub(14);
        }
    }

    fn build_lines(&mut self, keymap: &KeymapManager, view: &View, mode: &Mode) {
        self.lines.clear();

        self.lines.push(HelpLine::Header(format!(
            "Preset: {}",
            keymap.preset_name()
        )));
        self.lines.push(HelpLine::Header(
            "j/k: navigate  Enter: execute  Esc: close".to_string(),
        ));

        push_section_lines(
            &mut self.lines,
            "Global",
            keymap.bindings_for(KeyContext::Global),
        );
        push_section_lines(
            &mut self.lines,
            section_title(view),
            keymap.bindings_for(section_context(view)),
        );

        if *mode == Mode::Command {
            self.lines.push(HelpLine::Header(String::new()));
            self.lines.push(HelpLine::Header("Command mode".to_string()));
            self.lines.push(HelpLine::Header(
                "  :keymap vim | :keymap helix".to_string(),
            ));
        }
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        keymap: &KeymapManager,
        _view: &View,
        _mode: &Mode,
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
        let content_lines = self.render_lines();
        let paragraph = Paragraph::new(content_lines)
            .style(popup_style().fg(self.styles.text_primary.fg.unwrap_or(Color::White)))
            .scroll((self.scroll, 0));
        f.render_widget(paragraph, inner);
    }

    fn render_lines(&self) -> Vec<Line<'static>> {
        self.lines
            .iter()
            .enumerate()
            .map(|(i, line)| match line {
                HelpLine::Header(text) => {
                    if text.starts_with("Preset:") || text == "Command mode" {
                        Line::from(Span::styled(
                            text.clone(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(Span::styled(text.clone(), self.styles.text_secondary))
                    }
                }
                HelpLine::Binding(b) => {
                    let is_selected = i == self.selected && b.action.is_some();
                    let prefix = if is_selected { "> " } else { "  " };
                    let key_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    };
                    let desc_style = if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(vec![
                        Span::styled(prefix.to_string(), desc_style),
                        Span::styled(format!("{:<12}", b.key), key_style),
                        Span::styled(b.description.to_string(), desc_style),
                    ])
                }
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn debug_lines(&self) -> Vec<Line<'static>> {
        self.render_lines()
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
        overlay.open(&keymap, &View::Main, &Mode::Normal);

        let lines = overlay.debug_lines();
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
        overlay.open(&keymap, &View::Main, &Mode::Normal);

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

fn push_section_lines(lines: &mut Vec<HelpLine>, title: &str, hints: Vec<KeyBindingHint>) {
    lines.push(HelpLine::Header(title.to_string()));
    for hint in hints {
        lines.push(HelpLine::Binding(hint));
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
