use anyhow::{Context, Result};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde::{Deserialize, Serialize};

use crate::{
    config::{ConfigFile, LayoutConfig},
    gitops::Repository,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneId {
    Files,
    Branches,
    Log,
    Rebase,
    Console,
    Stash,
    Remote,
    Shelve,
}

impl PaneId {
    pub const ALL: [PaneId; 8] = [
        PaneId::Files,
        PaneId::Branches,
        PaneId::Log,
        PaneId::Rebase,
        PaneId::Console,
        PaneId::Stash,
        PaneId::Remote,
        PaneId::Shelve,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PaneId::Files => "Files",
            PaneId::Branches => "Branches",
            PaneId::Log => "Log",
            PaneId::Rebase => "Rebase",
            PaneId::Console => "Console",
            PaneId::Stash => "Stash",
            PaneId::Remote => "Remote",
            PaneId::Shelve => "Shelve",
        }
    }

    pub fn index(self) -> usize {
        match self {
            PaneId::Files => 0,
            PaneId::Branches => 1,
            PaneId::Log => 2,
            PaneId::Rebase => 3,
            PaneId::Console => 4,
            PaneId::Stash => 5,
            PaneId::Remote => 6,
            PaneId::Shelve => 7,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn cell(self) -> (usize, usize) {
        match self {
            PaneId::Files => (0, 0),
            PaneId::Branches => (1, 0),
            PaneId::Log => (2, 0),
            PaneId::Rebase => (3, 0),
            PaneId::Console => (0, 1),
            PaneId::Stash => (1, 1),
            PaneId::Remote => (2, 1),
            PaneId::Shelve => (3, 1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutState {
    #[serde(default = "default_active_index")]
    pub active_index: usize,
    #[serde(default = "default_columns")]
    pub columns: [u16; 4],
    #[serde(default = "default_rows")]
    pub rows: [u16; 2],
}

fn default_active_index() -> usize {
    0
}

fn default_columns() -> [u16; 4] {
    [28, 24, 24, 24]
}

fn default_rows() -> [u16; 2] {
    [62, 38]
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            active_index: default_active_index(),
            columns: default_columns(),
            rows: default_rows(),
        }
    }
}

impl LayoutState {
    pub fn load(repo: &Repository, defaults: &LayoutConfig) -> Self {
        match repo.git_config_get("cogit.layout") {
            Ok(Some(raw)) => toml::from_str::<LayoutConfig>(&raw)
                .map(Into::into)
                .unwrap_or_else(|_| defaults.clone().into()),
            _ => defaults.clone().into(),
        }
    }

    pub fn active_pane(&self) -> PaneId {
        PaneId::from_index(self.active_index)
    }

    pub fn active_label(&self) -> &'static str {
        self.active_pane().label()
    }

    pub fn set_active(&mut self, pane: PaneId) {
        self.active_index = pane.index();
    }

    pub fn next_pane(&mut self) {
        self.active_index = (self.active_index + 1) % PaneId::ALL.len();
    }

    pub fn prev_pane(&mut self) {
        self.active_index = if self.active_index == 0 {
            PaneId::ALL.len() - 1
        } else {
            self.active_index - 1
        };
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn grow_width(&mut self) {
        self.adjust_width(1);
    }

    pub fn shrink_width(&mut self) {
        self.adjust_width(-1);
    }

    pub fn grow_height(&mut self) {
        self.adjust_height(1);
    }

    pub fn shrink_height(&mut self) {
        self.adjust_height(-1);
    }

    pub fn persist_local(&self, repo: &Repository) -> Result<()> {
        let value = toml::to_string(&LayoutConfig::from(self.clone()))
            .context("failed to serialize layout")?;
        repo.git_config_set("cogit.layout", &value)
    }

    pub fn clear_local(&self, repo: &Repository) -> Result<()> {
        repo.git_config_unset("cogit.layout")
    }

    pub fn persist_global(&self, config_file: &mut ConfigFile) -> Result<()> {
        config_file.config.layout = self.clone().into();
        config_file.save()
    }

    pub fn pane_rects(&self, area: Rect) -> [Rect; 8] {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                self.rows
                    .iter()
                    .copied()
                    .map(Constraint::Percentage)
                    .collect::<Vec<_>>(),
            )
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                self.columns
                    .iter()
                    .copied()
                    .map(Constraint::Percentage)
                    .collect::<Vec<_>>(),
            )
            .split(rows[0]);
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                self.columns
                    .iter()
                    .copied()
                    .map(Constraint::Percentage)
                    .collect::<Vec<_>>(),
            )
            .split(rows[1]);

        [
            top[0], top[1], top[2], top[3], bottom[0], bottom[1], bottom[2], bottom[3],
        ]
    }

    fn adjust_width(&mut self, delta: i16) {
        let (col_idx, _) = self.active_pane().cell();
        let neighbor = if delta >= 0 {
            (col_idx + 1).min(self.columns.len() - 1)
        } else {
            col_idx.saturating_sub(1)
        };
        if neighbor == col_idx {
            return;
        }
        shift_weights(&mut self.columns, col_idx, neighbor, delta);
    }

    fn adjust_height(&mut self, delta: i16) {
        let (_, row_idx) = self.active_pane().cell();
        let neighbor = if delta >= 0 {
            (row_idx + 1).min(self.rows.len() - 1)
        } else {
            row_idx.saturating_sub(1)
        };
        if neighbor == row_idx {
            return;
        }
        shift_weights(&mut self.rows, row_idx, neighbor, delta);
    }
}

fn shift_weights<const N: usize>(weights: &mut [u16; N], from: usize, to: usize, delta: i16) {
    if from == to {
        return;
    }

    let step = delta.unsigned_abs().max(1) as u16;
    if delta > 0 {
        let available = weights[to].saturating_sub(1);
        let actual = step.min(available);
        if actual == 0 {
            return;
        }
        weights[from] = weights[from].saturating_add(actual);
        weights[to] = weights[to].saturating_sub(actual);
    } else {
        let available = weights[from].saturating_sub(1);
        let actual = step.min(available);
        if actual == 0 {
            return;
        }
        weights[from] = weights[from].saturating_sub(actual);
        weights[to] = weights[to].saturating_add(actual);
    }
}

impl From<LayoutConfig> for LayoutState {
    fn from(value: LayoutConfig) -> Self {
        Self {
            active_index: value.active_index,
            columns: value.columns,
            rows: value.rows,
        }
    }
}

impl From<LayoutState> for LayoutConfig {
    fn from(value: LayoutState) -> Self {
        Self {
            active_index: value.active_index,
            columns: value.columns,
            rows: value.rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_panes_in_row_major_order() {
        let mut layout = LayoutState::default();
        assert_eq!(layout.active_pane(), PaneId::Files);
        layout.next_pane();
        assert_eq!(layout.active_pane(), PaneId::Branches);
        layout.next_pane();
        assert_eq!(layout.active_pane(), PaneId::Log);
        layout.prev_pane();
        assert_eq!(layout.active_pane(), PaneId::Branches);
    }

    #[test]
    fn adjusts_column_weights() {
        let mut layout = LayoutState::default();
        let before = layout.columns;
        layout.grow_width();
        assert_ne!(layout.columns, before);
        assert_eq!(
            layout.columns.iter().sum::<u16>(),
            before.iter().sum::<u16>()
        );
    }

    #[test]
    fn serializes_to_layout_config_toml() {
        let layout = LayoutState {
            active_index: 3,
            columns: [30, 20, 25, 25],
            rows: [55, 45],
        };
        let rendered = toml::to_string(&LayoutConfig::from(layout.clone())).unwrap();
        let parsed: LayoutConfig = toml::from_str(&rendered).unwrap();
        assert_eq!(parsed.active_index, 3);
        assert_eq!(parsed.columns, [30, 20, 25, 25]);
        assert_eq!(parsed.rows, [55, 45]);
        assert_eq!(LayoutState::from(parsed), layout);
    }
}
