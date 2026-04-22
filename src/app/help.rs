use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::styles::Styles;

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

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_w = (area.width.saturating_mul(4) / 5).max(60);
        let popup_h = (area.height.saturating_mul(4) / 5).max(16);
        let popup_x = (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = (area.height.saturating_sub(popup_h)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_w, popup_h);

        let clear = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear, popup_area);

        let border = Block::default()
            .borders(Borders::ALL)
            .title(" Help  Esc/q/? to close  j/k PgUp/PgDn G/g to scroll ")
            .border_style(self.styles.border_active);
        f.render_widget(border, popup_area);

        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        let paragraph = Paragraph::new(self.content())
            .style(self.styles.text_primary)
            .scroll((self.scroll, 0));
        f.render_widget(paragraph, inner);
    }

    fn content(&self) -> Vec<Line<'static>> {
        let heading = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let key = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let text = self.styles.text_primary;
        let muted = self.styles.text_secondary;

        vec![
            Line::from(vec![Span::styled("cogit help", heading)]),
            Line::from(""),
            Line::from(vec![Span::styled("Close: ", muted), Span::styled("Esc  q  ?", key)]),
            Line::from(vec![
                Span::styled("Scroll: ", muted),
                Span::styled("j/k  PageUp/PageDown  G/g", key),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Main view", heading)]),
            Line::from(vec![Span::styled("  1", key), Span::styled(" branches", text)]),
            Line::from(vec![Span::styled("  2", key), Span::styled(" log", text)]),
            Line::from(vec![Span::styled("  4", key), Span::styled(" stash / shelve", text)]),
            Line::from(vec![Span::styled("  :", key), Span::styled(" command line", text)]),
            Line::from(vec![Span::styled("  ?", key), Span::styled(" help overlay", text)]),
            Line::from(vec![Span::styled("  q", key), Span::styled(" quit app", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" move in file list", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" open diff popup", text)]),
            Line::from(vec![Span::styled("  Space", key), Span::styled(" stage or unstage selected file", text)]),
            Line::from(vec![Span::styled("  s", key), Span::styled(" stage selected file", text)]),
            Line::from(vec![Span::styled("  S", key), Span::styled(" stage all files", text)]),
            Line::from(vec![Span::styled("  u", key), Span::styled(" unstage selected file", text)]),
            Line::from(vec![Span::styled("  U", key), Span::styled(" unstage all files", text)]),
            Line::from(vec![Span::styled("  c", key), Span::styled(" open commit dialog", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Command mode", heading)]),
            Line::from(vec![Span::styled("  :help", key), Span::styled(" show this help", text)]),
            Line::from(vec![Span::styled("  :commit", key), Span::styled(" open commit dialog", text)]),
            Line::from(vec![Span::styled("  :commit -m \"msg\"", key), Span::styled(" commit immediately", text)]),
            Line::from(vec![Span::styled("  :stage / :unstage", key), Span::styled(" stage or unstage selected file", text)]),
            Line::from(vec![Span::styled("  :stageall / :unstageall", key), Span::styled(" stage or unstage everything", text)]),
            Line::from(vec![Span::styled("  :discard", key), Span::styled(" discard selected file changes", text)]),
            Line::from(vec![Span::styled("  :stash", key), Span::styled(" open stash/shelve view", text)]),
            Line::from(vec![Span::styled("  :stashpop", key), Span::styled(" pop a stash entry", text)]),
            Line::from(vec![Span::styled("  :log", key), Span::styled(" open log view", text)]),
            Line::from(vec![Span::styled("  :branches / :branch", key), Span::styled(" open branch view", text)]),
            Line::from(vec![Span::styled("  :push / :fetch", key), Span::styled(" sync with remotes", text)]),
            Line::from(vec![Span::styled("  :checkout <branch>", key), Span::styled(" checkout a branch", text)]),
            Line::from(vec![Span::styled("  :amend", key), Span::styled(" amend the last commit", text)]),
            Line::from(vec![Span::styled("  :tag <name>", key), Span::styled(" create a tag", text)]),
            Line::from(vec![Span::styled("  :tag / :tags", key), Span::styled(" list all tags", text)]),
            Line::from(vec![Span::styled("  :q / :q!", key), Span::styled(" quit", text)]),
            Line::from(vec![Span::styled("  :w / :wq / :c", key), Span::styled(" stage or commit shortcuts", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Log panel", heading)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" move through commits", text)]),
            Line::from(vec![Span::styled("  Ctrl+u / Ctrl+d", key), Span::styled(" page up/down", text)]),
            Line::from(vec![Span::styled("  /", key), Span::styled(" search commits", text)]),
            Line::from(vec![Span::styled("  y", key), Span::styled(" copy commit hash", text)]),
            Line::from(vec![Span::styled("  c", key), Span::styled(" cherry-pick selected commit", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Branch panel", heading)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" move through branches", text)]),
            Line::from(vec![Span::styled("  Ctrl+u / Ctrl+d", key), Span::styled(" page up/down", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" checkout selected branch", text)]),
            Line::from(vec![Span::styled("  n", key), Span::styled(" create branch", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" delete selected branch", text)]),
            Line::from(vec![Span::styled("  f", key), Span::styled(" fetch all remotes", text)]),
            Line::from(vec![Span::styled("  p", key), Span::styled(" push current branch", text)]),
            Line::from(vec![Span::styled("  P", key), Span::styled(" pull current branch", text)]),
            Line::from(vec![Span::styled("  m", key), Span::styled(" merge selected branch", text)]),
            Line::from(vec![Span::styled("  r", key), Span::styled(" rebase onto selected branch", text)]),
            Line::from(vec![Span::styled("  /", key), Span::styled(" search branches", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Stash / Shelve", heading)]),
            Line::from(vec![Span::styled("  Tab", key), Span::styled(" switch stash / shelve tab", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" move selection", text)]),
            Line::from(vec![Span::styled("  Ctrl+u / Ctrl+d", key), Span::styled(" page up/down", text)]),
            Line::from(vec![Span::styled("Stash tab:", muted)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" pop selected stash entry", text)]),
            Line::from(vec![Span::styled("  a", key), Span::styled(" apply selected stash entry", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" drop selected stash entry", text)]),
            Line::from(vec![Span::styled("  s", key), Span::styled(" create a new stash", text)]),
            Line::from(vec![Span::styled("Shelve tab:", muted)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" apply selected shelve entry", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" delete selected shelve entry", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
        ]
    }
}
