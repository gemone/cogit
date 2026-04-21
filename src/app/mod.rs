pub mod cmdline;
pub mod help;
pub mod keymap;
pub mod styles;

use std::io;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Clear, Widget},
    Terminal,
};

use crate::gitops::{GitError, Repo};
use crate::panels::{
    Action, CmdbarPanel, DiffViewerPanel, FileListPanel, Mode, Panel, SidebarPanel,
};

use cmdline::Cmdline;
use help::Help;
use keymap::KeyMap;
use styles::Styles;

pub struct App {
    repo: Repo,
    panels: Vec<Box<dyn Panel>>,
    focus_idx: usize,
    mode: Mode,
    styles: Styles,
    keymap: KeyMap,
    cmdline: Cmdline,
    help: Help,
    should_quit: bool,
    size: Rect,
}

impl App {
    pub fn new(repo: Repo) -> Result<Self, GitError> {
        let styles = Styles::dark();
        let keymap = KeyMap::default();
        let mut cmdbar = CmdbarPanel::new();
        let mut sidebar = SidebarPanel::new();
        let mut filelist = FileListPanel::new();
        let mut diffviewer = DiffViewerPanel::new();

        let mut repo = repo;
        cmdbar.refresh(&mut repo)?;
        sidebar.refresh(&mut repo)?;
        filelist.refresh(&mut repo)?;
        diffviewer.refresh(&mut repo)?;

        let mut panels: Vec<Box<dyn Panel>> = vec![
            Box::new(cmdbar),
            Box::new(sidebar),
            Box::new(filelist),
            Box::new(diffviewer),
        ];

        panels[1].focus();

        Ok(Self {
            repo,
            panels,
            focus_idx: 1,
            mode: Mode::Normal,
            styles,
            keymap,
            cmdline: Cmdline::new(),
            help: Help::new(),
            should_quit: false,
            size: Rect::default(),
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), anyhow::Error> {
        self.size = Rect::new(0, 0, terminal.size()?.width, terminal.size()?.height);

        while !self.should_quit {
            terminal.draw(|f: &mut ratatui::Frame| {
                self.size = f.area();
                if let Err(e) = self.draw_frame(f) {
                    let _ = e;
                }
            })?;

            let event = event::read().context("read terminal event")?;
            if let Err(e) = self.handle_event(event) {
                // For now we ignore panel errors in the loop; in P3 we may show a toast
                let _ = e;
            }
        }

        Ok(())
    }

    fn draw_frame(&mut self, f: &mut ratatui::Frame) -> Result<(), anyhow::Error> {
        let area = f.area();
        let buf = f.buffer_mut();

        // Layout: top bar + middle + bottom status
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);

        let top_area = main_chunks[0];
        let middle_area = main_chunks[1];
        let bottom_area = main_chunks[2];

        // Top bar (cmdbar)
        self.panels[0].render(top_area, buf, &self.styles);

        // Middle: sidebar | filelist | diffviewer
        let mid_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ])
            .split(middle_area);

        self.panels[1].render(mid_chunks[0], buf, &self.styles);
        self.panels[2].render(mid_chunks[1], buf, &self.styles);
        self.panels[3].render(mid_chunks[2], buf, &self.styles);

        // Bottom status
        let mode_text = format!("[{}]", mode_label(self.mode));
        let status = ratatui::widgets::Paragraph::new(mode_text)
            .style(self.styles.cmdbar_active);
        status.render(bottom_area, buf);

        // Command palette overlay
        if self.cmdline.visible {
            let cmd_area = centered_rect(60, 20, area);
            Clear.render(cmd_area, buf);
            self.cmdline.render(cmd_area, buf, &self.styles);
        }

        // Help overlay
        self.help.render(area, buf, &self.styles, self.mode);

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), GitError> {
        match event {
            Event::Resize(cols, rows) => {
                self.size = Rect::new(0, 0, cols, rows);
            }
            Event::Key(key) => {
                if self.help.visible {
                    self.help.hide();
                    return Ok(());
                }

                if self.mode == Mode::Command && self.cmdline.visible {
                    self.handle_command_key(key);
                    return Ok(());
                }

                let panel = self.focused_panel_name();
                let action = self.keymap.resolve(self.mode, panel, key);
                self.dispatch(action, key)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let _ = self.cmdline.submit();
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.cmdline.close();
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                self.cmdline.push_char(c);
            }
            KeyCode::Backspace => {
                self.cmdline.backspace();
            }
            KeyCode::Left => {
                self.cmdline.move_cursor_left();
            }
            KeyCode::Right => {
                self.cmdline.move_cursor_right();
            }
            _ => {}
        }
    }

    fn dispatch(&mut self, action: Action, key: KeyEvent) -> Result<(), GitError> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::FocusSidebar => self.set_focus(1),
            Action::FocusFilelist => self.set_focus(2),
            Action::FocusDiff => self.set_focus(3),
            Action::CommandPalette => {
                self.cmdline.open();
                self.mode = Mode::Command;
            }
            Action::Help => {
                let panel = self.focused_panel_name().to_string();
                self.help.toggle(&panel);
            }
            Action::Refresh => {
                for panel in &mut self.panels {
                    panel.refresh(&mut self.repo)?;
                }
            }
            Action::EnterMode(m) => self.mode = m,
            Action::None => {
                // Pass unhandled keys to focused panel
                let idx = self.focus_idx;
                if let Some(a) = self.panels[idx].handle_key(key) {
                    return self.dispatch(a, key);
                }
            }
            _ => {
                // Panel-specific actions may be handled here or delegated
                let idx = self.focus_idx;
                if let Some(a) = self.panels[idx].handle_key(key) {
                    if a != Action::None {
                        return self.dispatch(a, key);
                    }
                }
            }
        }
        Ok(())
    }

    fn set_focus(&mut self, idx: usize) {
        if idx < self.panels.len() && idx != self.focus_idx {
            self.panels[self.focus_idx].blur();
            self.focus_idx = idx;
            self.panels[self.focus_idx].focus();
        }
    }

    fn focused_panel_name(&self) -> &str {
        self.panels.get(self.focus_idx).map(|p| p.title()).unwrap_or("")
    }
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Visual => "VISUAL",
        Mode::Command => "COMMAND",
        Mode::Insert => "INSERT",
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
