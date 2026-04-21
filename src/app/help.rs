use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Row, Table, Widget},
};

use crate::panels::Mode;

use super::styles::Styles;

pub struct Help {
    pub visible: bool,
    pub panel_name: String,
}

impl Help {
    pub fn new() -> Self {
        Self {
            visible: false,
            panel_name: String::new(),
        }
    }

    pub fn toggle(&mut self, panel_name: &str) {
        if self.visible && self.panel_name == panel_name {
            self.visible = false;
        } else {
            self.visible = true;
            self.panel_name = panel_name.to_string();
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, _styles: &Styles, mode: Mode) {
        if !self.visible {
            return;
        }

        let help_area = centered_rect(70, 70, area);
        Clear.render(help_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(format!(" Help — {} [{}] ", self.panel_name, mode_label(mode)));

        let rows = help_rows(&self.panel_name, mode);
        let table = Table::new(
            rows,
            &[Constraint::Percentage(30), Constraint::Percentage(70)],
        )
        .block(block)
        .header(Row::new(vec!["Key", "Action"]).style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ));

        Widget::render(table, help_area, buf);
    }
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "Normal",
        Mode::Visual => "Visual",
        Mode::Command => "Command",
        Mode::Insert => "Insert",
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

fn help_rows(panel: &str, _mode: Mode) -> Vec<Row<'static>> {
    let data: Vec<(&'static str, &'static str)> = match panel {
        "sidebar" => vec![
            ("j / k", "Move up/down"),
            ("c", "Checkout branch"),
            ("b", "New branch"),
            ("D", "Delete branch"),
            ("o", "Expand/collapse"),
            ("r", "Refresh"),
            ("Tab", "Next panel"),
            ("?", "Toggle help"),
            ("q", "Quit"),
        ],
        "filelist" => vec![
            ("j / k", "Move up/down"),
            ("Space / s", "Stage"),
            ("u", "Unstage"),
            ("a", "Stage all"),
            ("A", "Unstage all"),
            ("d", "Discard"),
            ("c", "Commit"),
            ("Enter", "Open diff"),
            ("i / I", "Toggle ignore"),
            ("U", "Toggle untracked"),
            ("/", "Search"),
            ("Tab", "Next panel"),
            ("?", "Toggle help"),
            ("q", "Quit"),
        ],
        "diff" => vec![
            ("j / k", "Move up/down"),
            ("s", "Stage hunk"),
            ("u", "Unstage hunk"),
            ("Tab", "Next panel"),
            ("?", "Toggle help"),
            ("q", "Quit"),
        ],
        _ => vec![
            ("Tab / S-Tab", "Next/prev panel"),
            ("1 / 2 / 3", "Focus sidebar/filelist/diff"),
            (":", "Command palette"),
            ("?", "Toggle help"),
            ("q", "Quit"),
        ],
    };

    data.into_iter()
        .map(|(k, v)| Row::new(vec![Cell::from(k), Cell::from(v)]))
        .collect()
}
