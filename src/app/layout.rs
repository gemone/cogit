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
    Stash,
    Log,
    Rebase,
    Remote,
    Shelve,
    Console,
}

impl PaneId {
    pub const ALL: [PaneId; 8] = [
        PaneId::Files,
        PaneId::Branches,
        PaneId::Stash,
        PaneId::Log,
        PaneId::Rebase,
        PaneId::Remote,
        PaneId::Shelve,
        PaneId::Console,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PaneId::Files => "Files",
            PaneId::Branches => "Branches",
            PaneId::Stash => "Stash",
            PaneId::Log => "Log",
            PaneId::Rebase => "Rebase",
            PaneId::Remote => "Remote",
            PaneId::Shelve => "Shelve",
            PaneId::Console => "Console",
        }
    }

    pub fn index(self) -> usize {
        match self {
            PaneId::Files => 0,
            PaneId::Branches => 1,
            PaneId::Stash => 2,
            PaneId::Log => 3,
            PaneId::Rebase => 4,
            PaneId::Remote => 5,
            PaneId::Shelve => 6,
            PaneId::Console => 7,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    fn column_group(self) -> ColumnGroup {
        match self {
            PaneId::Files | PaneId::Branches | PaneId::Stash => ColumnGroup::Left,
            PaneId::Log | PaneId::Rebase => ColumnGroup::Center,
            PaneId::Remote | PaneId::Shelve => ColumnGroup::Right,
            PaneId::Console => ColumnGroup::Bottom,
        }
    }

    fn stack_index(self) -> Option<usize> {
        match self {
            PaneId::Files => Some(0),
            PaneId::Branches => Some(1),
            PaneId::Stash => Some(2),
            PaneId::Log => Some(0),
            PaneId::Rebase => Some(1),
            PaneId::Remote => Some(0),
            PaneId::Shelve => Some(1),
            PaneId::Console => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnGroup {
    Left,
    Center,
    Right,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutState {
    #[serde(default = "default_active_index")]
    pub active_index: usize,
    #[serde(default = "default_vertical")]
    pub vertical: [u16; 2],
    #[serde(default = "default_columns")]
    pub columns: [u16; 3],
    #[serde(default = "default_left_rows")]
    pub left_rows: [u16; 3],
    #[serde(default = "default_center_rows")]
    pub center_rows: [u16; 2],
    #[serde(default = "default_right_rows")]
    pub right_rows: [u16; 2],
}

fn default_active_index() -> usize {
    0
}

fn default_vertical() -> [u16; 2] {
    [82, 18]
}

fn default_columns() -> [u16; 3] {
    [28, 44, 28]
}

fn default_left_rows() -> [u16; 3] {
    [48, 28, 24]
}

fn default_center_rows() -> [u16; 2] {
    [68, 32]
}

fn default_right_rows() -> [u16; 2] {
    [52, 48]
}

const MIN_COLUMN_WEIGHT: u16 = 12;
const MIN_STACK_WEIGHT: u16 = 13;
const MIN_VERTICAL_WEIGHT: u16 = 12;

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            active_index: default_active_index(),
            vertical: default_vertical(),
            columns: default_columns(),
            left_rows: default_left_rows(),
            center_rows: default_center_rows(),
            right_rows: default_right_rows(),
        }
    }
}

impl LayoutState {
    pub fn load(repo: &Repository, defaults: &LayoutConfig) -> Self {
        match repo.git_config_get("cogit.layout") {
            Ok(Some(raw)) => parse_local_layout(&raw)
                .map(Into::into)
                .unwrap_or_else(|| defaults.clone().into()),
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
        self.adjust_width(2);
    }

    pub fn shrink_width(&mut self) {
        self.adjust_width(-2);
    }

    pub fn grow_height(&mut self) {
        self.adjust_height(2);
    }

    pub fn shrink_height(&mut self) {
        self.adjust_height(-2);
    }

    pub fn persist_local(&self, repo: &Repository) -> Result<()> {
        let value = serde_json::to_string(&LayoutConfig::from(self.clone()))
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
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(percentages(&self.vertical))
            .split(area);

        let top = vertical[0];
        let console = vertical[1];

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(percentages(&self.columns))
            .split(top);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints(percentages(&self.left_rows))
            .split(columns[0]);
        let center = Layout::default()
            .direction(Direction::Vertical)
            .constraints(percentages(&self.center_rows))
            .split(columns[1]);
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(percentages(&self.right_rows))
            .split(columns[2]);

        [
            left[0], left[1], left[2], center[0], center[1], right[0], right[1], console,
        ]
    }

    fn adjust_width(&mut self, delta: i16) {
        match self.active_pane().column_group() {
            ColumnGroup::Left => {
                shift_weights_with_min(&mut self.columns, 0, 1, delta, MIN_COLUMN_WEIGHT)
            }
            ColumnGroup::Center => {
                shift_weights_with_min(&mut self.columns, 1, 2, delta, MIN_COLUMN_WEIGHT)
            }
            ColumnGroup::Right => {
                shift_weights_with_min(&mut self.columns, 2, 1, delta, MIN_COLUMN_WEIGHT)
            }
            ColumnGroup::Bottom => {}
        }
    }

    fn adjust_height(&mut self, delta: i16) {
        let pane = self.active_pane();
        match pane.column_group() {
            ColumnGroup::Left => {
                if let Some(index) = pane.stack_index() {
                    adjust_stack(&mut self.left_rows, index, delta);
                }
            }
            ColumnGroup::Center => {
                if let Some(index) = pane.stack_index() {
                    adjust_stack(&mut self.center_rows, index, delta);
                }
            }
            ColumnGroup::Right => {
                if let Some(index) = pane.stack_index() {
                    adjust_stack(&mut self.right_rows, index, delta);
                }
            }
            ColumnGroup::Bottom => {
                shift_weights_with_min(&mut self.vertical, 1, 0, delta, MIN_VERTICAL_WEIGHT)
            }
        }
    }
}

fn percentages<const N: usize>(weights: &[u16; N]) -> Vec<Constraint> {
    weights
        .iter()
        .copied()
        .map(Constraint::Percentage)
        .collect()
}

fn parse_local_layout(raw: &str) -> Option<LayoutConfig> {
    serde_json::from_str(raw)
        .or_else(|_| toml::from_str(raw))
        .ok()
}

fn adjust_stack<const N: usize>(weights: &mut [u16; N], index: usize, delta: i16) {
    if N < 2 {
        return;
    }

    let neighbor = if delta >= 0 {
        if index + 1 < N { index + 1 } else { index - 1 }
    } else if index > 0 {
        index - 1
    } else {
        1
    };

    shift_weights_with_min(weights, index, neighbor, delta, MIN_STACK_WEIGHT);
}

fn shift_weights_with_min<const N: usize>(
    weights: &mut [u16; N],
    from: usize,
    to: usize,
    delta: i16,
    min_weight: u16,
) {
    if from == to {
        return;
    }

    let step = delta.unsigned_abs().max(1) as u16;
    if delta > 0 {
        let available = weights[to].saturating_sub(min_weight);
        let actual = step.min(available);
        if actual == 0 {
            return;
        }
        weights[from] = weights[from].saturating_add(actual);
        weights[to] = weights[to].saturating_sub(actual);
    } else {
        let available = weights[from].saturating_sub(min_weight);
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
            vertical: value.vertical,
            columns: value.columns,
            left_rows: value.left_rows,
            center_rows: value.center_rows,
            right_rows: value.right_rows,
        }
    }
}

impl From<LayoutState> for LayoutConfig {
    fn from(value: LayoutState) -> Self {
        Self {
            active_index: value.active_index,
            vertical: value.vertical,
            columns: value.columns,
            left_rows: value.left_rows,
            center_rows: value.center_rows,
            right_rows: value.right_rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_panes_in_visual_order() {
        let mut layout = LayoutState::default();
        assert_eq!(layout.active_pane(), PaneId::Files);
        layout.next_pane();
        assert_eq!(layout.active_pane(), PaneId::Branches);
        layout.next_pane();
        assert_eq!(layout.active_pane(), PaneId::Stash);
        layout.next_pane();
        assert_eq!(layout.active_pane(), PaneId::Log);
        layout.prev_pane();
        assert_eq!(layout.active_pane(), PaneId::Stash);
    }

    #[test]
    fn adjusts_column_weights() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Log);
        let before = layout.columns;
        layout.grow_width();
        assert_ne!(layout.columns, before);
        assert_eq!(
            layout.columns.iter().sum::<u16>(),
            before.iter().sum::<u16>()
        );
    }

    #[test]
    fn adjusts_console_height() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Console);
        let before = layout.vertical;
        layout.grow_height();
        assert_ne!(layout.vertical, before);
        assert_eq!(
            layout.vertical.iter().sum::<u16>(),
            before.iter().sum::<u16>()
        );
    }

    #[test]
    fn console_spans_full_width() {
        let layout = LayoutState::default();
        let rects = layout.pane_rects(Rect::new(0, 0, 120, 40));
        let console = rects[PaneId::Console.index()];
        assert_eq!(console.x, 0);
        assert_eq!(console.width, 120);
        assert!(console.y > rects[PaneId::Files.index()].y);
    }

    #[test]
    fn serializes_to_layout_config_toml() {
        let layout = LayoutState {
            active_index: 4,
            vertical: [84, 16],
            columns: [30, 42, 28],
            left_rows: [45, 30, 25],
            center_rows: [70, 30],
            right_rows: [55, 45],
        };
        let rendered = toml::to_string(&LayoutConfig::from(layout.clone())).unwrap();
        let parsed: LayoutConfig = toml::from_str(&rendered).unwrap();
        assert_eq!(parsed.active_index, 4);
        assert_eq!(parsed.vertical, [84, 16]);
        assert_eq!(parsed.columns, [30, 42, 28]);
        assert_eq!(parsed.left_rows, [45, 30, 25]);
        assert_eq!(parsed.center_rows, [70, 30]);
        assert_eq!(parsed.right_rows, [55, 45]);
        assert_eq!(LayoutState::from(parsed), layout);
    }

    #[test]
    fn parses_legacy_toml_local_layout() {
        let raw = "active_index = 5\ncolumns = [28, 24, 24, 24]\nrows = [62, 38]\n";
        let parsed = parse_local_layout(raw).unwrap();
        assert_eq!(parsed.active_index, 2);
        assert_eq!(parsed.vertical, [88, 12]);
        assert_eq!(parsed.columns, [28, 48, 24]);
    }

    #[test]
    fn parses_json_local_layout() {
        let raw = r#"{"active_index":4,"vertical":[84,16],"columns":[30,42,28],"left_rows":[45,30,25],"center_rows":[70,30],"right_rows":[55,45]}"#;
        let parsed = parse_local_layout(raw).unwrap();
        assert_eq!(parsed.active_index, 4);
        assert_eq!(parsed.vertical, [84, 16]);
        assert_eq!(parsed.columns, [30, 42, 28]);
        assert_eq!(parsed.left_rows, [45, 30, 25]);
        assert_eq!(parsed.center_rows, [70, 30]);
        assert_eq!(parsed.right_rows, [55, 45]);
    }
}
