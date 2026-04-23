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
            Line::from(vec![Span::styled("Close:", muted), Span::styled("Esc  q  ?", key)]),
            Line::from(vec![
                Span::styled("Scroll:", muted),
                Span::styled("j/k  PageUp/PageDown  G/g", key),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Main view", heading)]),
            Line::from(vec![Span::styled("  q", key), Span::styled(" quit app", text)]),
            Line::from(vec![Span::styled("  s", key), Span::styled(" stage selected file", text)]),
            Line::from(vec![Span::styled("  S", key), Span::styled(" stage all files", text)]),
            Line::from(vec![Span::styled("  u", key), Span::styled(" unstage selected file", text)]),
            Line::from(vec![Span::styled("  U", key), Span::styled(" unstage all files", text)]),
            Line::from(vec![Span::styled("  c", key), Span::styled(" open commit dialog", text)]),
            Line::from(vec![Span::styled("  Space", key), Span::styled(" stage/unstage selected file (toggle)", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" open diff popup for selected file", text)]),
            Line::from(vec![Span::styled("  1", key), Span::styled(" open branches panel", text)]),
            Line::from(vec![Span::styled("  2", key), Span::styled(" open log panel", text)]),
            Line::from(vec![Span::styled("  4", key), Span::styled(" open stash/shelve panel", text)]),
            Line::from(vec![Span::styled("  R", key), Span::styled(" open remote panel", text)]),
            Line::from(vec![Span::styled("  S", key), Span::styled(" open shelve panel", text)]),
            Line::from(vec![Span::styled("  :", key), Span::styled(" open command line", text)]),
            Line::from(vec![Span::styled("  ?", key), Span::styled(" show this help", text)]),
            Line::from(vec![Span::styled("  j/k, G/g, Ctrl+d/u", key), Span::styled(" navigate file list", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" discard changes to selected file", text)]),
            Line::from(vec![Span::styled("  D", key), Span::styled(" stage & discard (reset HEAD)", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("File list panel", heading)]),
            Line::from(vec![Span::styled("  s / S", key), Span::styled(" stage / stage all", text)]),
            Line::from(vec![Span::styled("  u / U", key), Span::styled(" unstage / unstage all", text)]),
            Line::from(vec![Span::styled("  c", key), Span::styled(" commit", text)]),
            Line::from(vec![Span::styled("  Space / Enter", key), Span::styled(" stage/unstage / open diff", text)]),
            Line::from(vec![Span::styled("  d / D", key), Span::styled(" discard / stage+discard", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" navigate", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Branch panel", heading)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" checkout selected branch", text)]),
            Line::from(vec![Span::styled("  n", key), Span::styled(" create new branch (dialog)", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" delete selected branch", text)]),
            Line::from(vec![Span::styled("  f", key), Span::styled(" fetch all remotes", text)]),
            Line::from(vec![Span::styled("  p", key), Span::styled(" push current branch", text)]),
            Line::from(vec![Span::styled("  P", key), Span::styled(" pull current branch", text)]),
            Line::from(vec![Span::styled("  m", key), Span::styled(" merge selected branch", text)]),
            Line::from(vec![Span::styled("  r", key), Span::styled(" rebase onto selected branch", text)]),
            Line::from(vec![Span::styled("  c/a/s", key), Span::styled(" rebase continue/abort/skip (during rebase)", text)]),
            Line::from(vec![Span::styled("  R", key), Span::styled(" rename selected branch (dialog)", text)]),
            Line::from(vec![Span::styled("  /", key), Span::styled(" search/filter branches", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" navigate", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Log panel", heading)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" show commit details", text)]),
            Line::from(vec![Span::styled("  y", key), Span::styled(" copy commit hash to clipboard", text)]),
            Line::from(vec![Span::styled("  c", key), Span::styled(" cherry-pick selected commit", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" diff selected commit vs parent", text)]),
            Line::from(vec![Span::styled("  p", key), Span::styled(" patch/apply selected commit", text)]),
            Line::from(vec![Span::styled("  r", key), Span::styled(" rebase onto this commit", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(vec![Span::styled("  j/k, G/g, /, Ctrl+d/u", key), Span::styled(" navigate/search", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Stash panel", heading)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" pop selected stash entry", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" drop selected stash entry", text)]),
            Line::from(vec![Span::styled("  p", key), Span::styled(" apply selected stash entry", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" navigate", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Shelve panel", heading)]),
            Line::from(vec![Span::styled("  n", key), Span::styled(" create new shelve (dialog)", text)]),
            Line::from(vec![Span::styled("  s", key), Span::styled(" toggle include staged", text)]),
            Line::from(vec![Span::styled("  p", key), Span::styled(" pop (apply & delete) selected shelve", text)]),
            Line::from(vec![Span::styled("  a", key), Span::styled(" apply selected shelve (keep)", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" drop selected shelve", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" view diff of selected shelve", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" navigate", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Remote panel", heading)]),
            Line::from(vec![Span::styled("  a", key), Span::styled(" add new remote (dialog)", text)]),
            Line::from(vec![Span::styled("  d", key), Span::styled(" delete selected remote", text)]),
            Line::from(vec![Span::styled("  r", key), Span::styled(" rename selected remote", text)]),
            Line::from(vec![Span::styled("  u", key), Span::styled(" fetch selected remote", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" show branches on selected remote", text)]),
            Line::from(vec![Span::styled("  q / Esc", key), Span::styled(" return to main view", text)]),
            Line::from(vec![Span::styled("  j/k, G/g", key), Span::styled(" navigate", text)]),
            Line::from(""),
            Line::from(vec![Span::styled("Command mode", heading)]),
            Line::from(vec![Span::styled("  Esc", key), Span::styled(" cancel and close command line", text)]),
            Line::from(vec![Span::styled("  Enter", key), Span::styled(" execute command", text)]),
            Line::from(vec![Span::styled("  Backspace", key), Span::styled(" delete character", text)]),
            Line::from(vec![Span::styled("  Arrow keys", key), Span::styled(" move cursor", text)]),
            Line::from(vec![Span::styled("  :help", key), Span::styled(" show this help", text)]),
            Line::from(vec![Span::styled("  :commit / :c", key), Span::styled(" open commit dialog", text)]),
            Line::from(vec![Span::styled("  :commit -m \"msg\"", key), Span::styled(" commit immediately", text)]),
            Line::from(vec![Span::styled("  :stage / :unstage", key), Span::styled(" stage or unstage selected file", text)]),
            Line::from(vec![Span::styled("  :stageall / :unstageall", key), Span::styled(" stage or unstage everything", text)]),
            Line::from(vec![Span::styled("  :discard", key), Span::styled(" discard selected file changes", text)]),
            Line::from(vec![Span::styled("  :stash", key), Span::styled(" open stash/shelve view", text)]),
            Line::from(vec![Span::styled("  :stashpop", key), Span::styled(" pop a stash entry", text)]),
            Line::from(vec![Span::styled("  :log", key), Span::styled(" open log view", text)]),
            Line::from(vec![Span::styled("  :branches / :branch", key), Span::styled(" open branch view", text)]),
            Line::from(vec![Span::styled("  :worktrees", key), Span::styled(" list all worktrees", text)]),
            Line::from(vec![Span::styled("  :worktree add <path> <branch>", key), Span::styled(" create a worktree", text)]),
            Line::from(vec![Span::styled("  :worktree remove <path>", key), Span::styled(" remove a worktree", text)]),
            Line::from(vec![Span::styled("  :push / :fetch", key), Span::styled(" sync with remotes", text)]),
            Line::from(vec![Span::styled("  :pull-rebase / :rebase-pull", key), Span::styled(" pull with rebase", text)]),
            Line::from(vec![Span::styled("  :checkout <branch>", key), Span::styled(" checkout a branch", text)]),
            Line::from(vec![Span::styled("  :rename-branch <old> <new>", key), Span::styled(" rename a branch", text)]),
            Line::from(vec![Span::styled("  :diff <ref1> <ref2>", key), Span::styled(" show diff between refs", text)]),
            Line::from(vec![Span::styled("  :amend", key), Span::styled(" amend the last commit", text)]),
            Line::from(vec![Span::styled("  :tag <name>", key), Span::styled(" create a tag", text)]),
            Line::from(vec![Span::styled("  :tag / :tags", key), Span::styled(" list all tags", text)]),
            Line::from(vec![Span::styled("  :reset [path] [soft|hard|mixed]", key), Span::styled(" reset HEAD (default: mixed)", text)]),
            Line::from(vec![Span::styled("  :reset-soft / :reset-hard / :reset-mixed", key), Span::styled(" reset with mode", text)]),
            Line::from(vec![Span::styled("  :wip", key), Span::styled(" create WIP commit (--no-verify)", text)]),
            Line::from(vec![Span::styled("  :q / :q!", key), Span::styled(" quit", text)]),
            Line::from(vec![Span::styled("  :w / :wq / :c", key), Span::styled(" stage or commit shortcuts", text)]),
        ]
    }
}
