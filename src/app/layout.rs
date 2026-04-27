use anyhow::{Context, Result};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        ConfigFile, LayoutConfig, default_layout_active_index, default_layout_center_rows,
        default_layout_columns, default_layout_hidden, default_layout_left_rows,
        default_layout_right_rows, default_layout_vertical,
    },
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

impl ColumnGroup {
    fn column_index(self) -> Option<usize> {
        match self {
            ColumnGroup::Left => Some(0),
            ColumnGroup::Center => Some(1),
            ColumnGroup::Right => Some(2),
            ColumnGroup::Bottom => None,
        }
    }
}

const LEFT_STACK_PANES: [PaneId; 3] = [PaneId::Files, PaneId::Branches, PaneId::Stash];
const CENTER_STACK_PANES: [PaneId; 2] = [PaneId::Log, PaneId::Rebase];
const RIGHT_STACK_PANES: [PaneId; 2] = [PaneId::Remote, PaneId::Shelve];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutState {
    #[serde(default = "default_layout_active_index")]
    pub active_index: usize,
    #[serde(default = "default_layout_vertical")]
    pub vertical: [u16; 2],
    #[serde(default = "default_layout_columns")]
    pub columns: [u16; 3],
    #[serde(default = "default_layout_left_rows")]
    pub left_rows: [u16; 3],
    #[serde(default = "default_layout_center_rows")]
    pub center_rows: [u16; 2],
    #[serde(default = "default_layout_right_rows")]
    pub right_rows: [u16; 2],
    #[serde(default = "default_layout_hidden")]
    pub hidden: [bool; 8],
}

const MIN_COLUMN_WEIGHT: u16 = 12;
const MIN_STACK_WEIGHT: u16 = 13;
const MIN_VERTICAL_WEIGHT: u16 = 12;

impl Default for LayoutState {
    fn default() -> Self {
        LayoutConfig::default().into()
    }
}

impl LayoutState {
    pub fn load(repo: &Repository, defaults: &LayoutConfig) -> Self {
        match repo.git_config_get("cogit.layout") {
            Ok(Some(raw)) => parse_local_layout(&raw)
                .map(Into::into)
                .map(Self::normalized)
                .unwrap_or_else(|| Self::from(defaults.clone()).normalized()),
            _ => Self::from(defaults.clone()).normalized(),
        }
    }

    pub fn active_pane(&self) -> PaneId {
        PaneId::from_index(self.active_index)
    }

    pub fn active_label(&self) -> &'static str {
        self.active_pane().label()
    }

    pub fn is_hidden(&self, pane: PaneId) -> bool {
        self.hidden[pane.index()]
    }

    pub fn show_all_panes(&mut self) {
        self.hidden = [false; 8];
    }

    pub fn hide_pane(&mut self, pane: PaneId) -> bool {
        if self.is_hidden(pane) {
            return true;
        }
        let visible_count = PaneId::ALL
            .iter()
            .filter(|pane_id| !self.is_hidden(**pane_id))
            .count();
        if visible_count <= 1 {
            return false;
        }

        self.hidden[pane.index()] = true;
        if self.active_pane() == pane {
            self.focus_next_visible();
        }
        true
    }

    fn focus_next_visible(&mut self) {
        for offset in 1..=PaneId::ALL.len() {
            let idx = (self.active_index + offset) % PaneId::ALL.len();
            let pane = PaneId::from_index(idx);
            if !self.is_hidden(pane) {
                self.active_index = idx;
                return;
            }
        }
    }

    pub fn normalized(mut self) -> Self {
        self.active_index %= PaneId::ALL.len();
        if self.hidden.iter().all(|hidden| *hidden) {
            self.hidden = [false; 8];
        }
        normalize_weights(&mut self.vertical, MIN_VERTICAL_WEIGHT);
        normalize_weights(&mut self.columns, MIN_COLUMN_WEIGHT);
        normalize_weights(&mut self.left_rows, MIN_STACK_WEIGHT);
        normalize_weights(&mut self.center_rows, MIN_STACK_WEIGHT);
        normalize_weights(&mut self.right_rows, MIN_STACK_WEIGHT);
        if self.is_hidden(self.active_pane()) {
            self.focus_next_visible();
        }
        self
    }

    pub fn show_pane(&mut self, pane: PaneId) {
        self.hidden[pane.index()] = false;
    }

    pub fn set_active(&mut self, pane: PaneId) {
        self.active_index = pane.index();
        if self.is_hidden(pane) {
            self.focus_next_visible();
        }
    }

    pub fn next_pane(&mut self) {
        self.focus_next_visible();
    }

    pub fn prev_pane(&mut self) {
        for offset in 1..=PaneId::ALL.len() {
            let idx = (self.active_index + PaneId::ALL.len() - offset) % PaneId::ALL.len();
            let pane = PaneId::from_index(idx);
            if !self.is_hidden(pane) {
                self.active_index = idx;
                return;
            }
        }
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
        let hidden_console = self.is_hidden(PaneId::Console);
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if hidden_console {
                vec![Constraint::Percentage(100), Constraint::Length(0)]
            } else {
                percentages(&self.vertical).to_vec()
            })
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

        let mut rects = [
            left[0], left[1], left[2], center[0], center[1], right[0], right[1], console,
        ];
        for pane in PaneId::ALL {
            if self.is_hidden(pane) {
                rects[pane.index()] = Rect::new(0, 0, 0, 0);
            }
        }
        rects
    }

    fn adjust_width(&mut self, delta: i16) {
        let group = self.active_pane().column_group();
        let from = group.column_index();
        let to = self
            .visible_column_neighbor(group)
            .and_then(ColumnGroup::column_index);

        if let (Some(from), Some(to)) = (from, to) {
            shift_weights_with_min(&mut self.columns, from, to, delta, MIN_COLUMN_WEIGHT);
        }
    }

    fn adjust_height(&mut self, delta: i16) {
        let pane = self.active_pane();
        let index = pane.stack_index();
        let neighbor = self.visible_stack_neighbor(pane);

        match pane.column_group() {
            ColumnGroup::Left => {
                if let (Some(index), Some(neighbor)) = (index, neighbor) {
                    shift_weights_with_min(
                        &mut self.left_rows,
                        index,
                        neighbor,
                        delta,
                        MIN_STACK_WEIGHT,
                    );
                }
            }
            ColumnGroup::Center => {
                if let (Some(index), Some(neighbor)) = (index, neighbor) {
                    shift_weights_with_min(
                        &mut self.center_rows,
                        index,
                        neighbor,
                        delta,
                        MIN_STACK_WEIGHT,
                    );
                }
            }
            ColumnGroup::Right => {
                if let (Some(index), Some(neighbor)) = (index, neighbor) {
                    shift_weights_with_min(
                        &mut self.right_rows,
                        index,
                        neighbor,
                        delta,
                        MIN_STACK_WEIGHT,
                    );
                }
            }
            ColumnGroup::Bottom => {
                shift_weights_with_min(&mut self.vertical, 1, 0, delta, MIN_VERTICAL_WEIGHT)
            }
        }
    }

    fn group_visible(&self, group: ColumnGroup) -> bool {
        match group {
            ColumnGroup::Left => LEFT_STACK_PANES.iter().any(|pane| !self.is_hidden(*pane)),
            ColumnGroup::Center => CENTER_STACK_PANES.iter().any(|pane| !self.is_hidden(*pane)),
            ColumnGroup::Right => RIGHT_STACK_PANES.iter().any(|pane| !self.is_hidden(*pane)),
            ColumnGroup::Bottom => !self.is_hidden(PaneId::Console),
        }
    }

    fn visible_column_neighbor(&self, group: ColumnGroup) -> Option<ColumnGroup> {
        let candidates = match group {
            ColumnGroup::Left => [Some(ColumnGroup::Center), Some(ColumnGroup::Right)],
            ColumnGroup::Center => [Some(ColumnGroup::Right), Some(ColumnGroup::Left)],
            ColumnGroup::Right => [Some(ColumnGroup::Center), Some(ColumnGroup::Left)],
            ColumnGroup::Bottom => [None, None],
        };

        candidates
            .into_iter()
            .flatten()
            .find(|candidate| self.group_visible(*candidate))
    }

    fn visible_stack_neighbor(&self, pane: PaneId) -> Option<usize> {
        let stack = match pane.column_group() {
            ColumnGroup::Left => &LEFT_STACK_PANES[..],
            ColumnGroup::Center => &CENTER_STACK_PANES[..],
            ColumnGroup::Right => &RIGHT_STACK_PANES[..],
            ColumnGroup::Bottom => return None,
        };
        let index = pane.stack_index()?;

        for candidate in (index + 1)..stack.len() {
            if !self.is_hidden(stack[candidate]) {
                return Some(candidate);
            }
        }
        for candidate in (0..index).rev() {
            if !self.is_hidden(stack[candidate]) {
                return Some(candidate);
            }
        }
        None
    }
}

fn percentages<const N: usize>(weights: &[u16; N]) -> [Constraint; N] {
    std::array::from_fn(|i| Constraint::Percentage(weights[i]))
}

fn parse_local_layout(raw: &str) -> Option<LayoutConfig> {
    serde_json::from_str(raw)
        .or_else(|_| toml::from_str(raw))
        .ok()
}

fn normalize_weights<const N: usize>(weights: &mut [u16; N], min_weight: u16) {
    if N == 0 {
        return;
    }

    let total = 100u32;
    let slots = N as u32;
    let min_weight_u32 = u32::from(min_weight);
    if min_weight_u32.saturating_mul(slots) > total {
        let base = (total / slots) as u16;
        let mut fallback = [0u16; N];
        fallback.fill(base);
        for weight in fallback.iter_mut().take((total % slots) as usize) {
            *weight += 1;
        }
        *weights = fallback;
        return;
    }

    let mut normalized = *weights;
    for weight in &mut normalized {
        *weight = (*weight).max(min_weight);
    }

    let mut sum: u32 = normalized.iter().map(|&weight| u32::from(weight)).sum();
    if sum == total {
        *weights = normalized;
        return;
    }

    if sum < total {
        normalized[0] = normalized[0].saturating_add((total - sum) as u16);
        *weights = normalized;
        return;
    }

    let mut excess = sum - total;
    while excess > 0 {
        let mut changed = false;
        for weight in &mut normalized {
            if excess == 0 {
                break;
            }
            if *weight > min_weight {
                *weight -= 1;
                excess -= 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    sum = normalized.iter().map(|&weight| u32::from(weight)).sum();
    if sum < total {
        normalized[0] = normalized[0].saturating_add((total - sum) as u16);
    }
    *weights = normalized;
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
            hidden: value.hidden,
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
            hidden: value.hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_state_default_matches_layout_config_default() {
        assert_eq!(LayoutState::default(), LayoutConfig::default().into());
    }

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
    fn center_column_growth_borrows_from_visible_right_neighbor_first() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Log);
        let before = layout.columns;

        layout.grow_width();

        assert_eq!(layout.columns[0], before[0]);
        assert!(layout.columns[1] > before[1]);
        assert!(layout.columns[2] < before[2]);
        assert_eq!(
            layout.columns.iter().sum::<u16>(),
            before.iter().sum::<u16>()
        );
    }

    #[test]
    fn center_column_growth_falls_back_to_left_when_right_column_hidden() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Log);
        assert!(layout.hide_pane(PaneId::Remote));
        assert!(layout.hide_pane(PaneId::Shelve));
        let before = layout.columns;

        layout.grow_width();

        assert!(layout.columns[0] < before[0]);
        assert!(layout.columns[1] > before[1]);
        assert_eq!(layout.columns[2], before[2]);
    }

    #[test]
    fn top_pane_growth_borrows_from_next_visible_pane_below() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Files);
        let before = layout.left_rows;

        layout.grow_height();

        assert!(layout.left_rows[0] > before[0]);
        assert!(layout.left_rows[1] < before[1]);
        assert_eq!(layout.left_rows[2], before[2]);
    }

    #[test]
    fn top_pane_growth_skips_hidden_neighbor_below() {
        let mut layout = LayoutState::default();
        layout.set_active(PaneId::Files);
        assert!(layout.hide_pane(PaneId::Branches));
        let before = layout.left_rows;

        layout.grow_height();

        assert!(layout.left_rows[0] > before[0]);
        assert_eq!(layout.left_rows[1], before[1]);
        assert!(layout.left_rows[2] < before[2]);
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
    fn normalizes_out_of_range_layout_state() {
        let layout = LayoutState {
            active_index: PaneId::ALL.len() + 3,
            vertical: [0, 0],
            columns: [99, 1, 0],
            left_rows: [0, 0, 100],
            center_rows: [100, 0],
            right_rows: [1, 99],
            hidden: [false; 8],
        }
        .normalized();

        assert_eq!(layout.active_pane(), PaneId::Log);
        assert_eq!(layout.vertical.iter().sum::<u16>(), 100);
        assert_eq!(layout.columns.iter().sum::<u16>(), 100);
        assert_eq!(layout.left_rows.iter().sum::<u16>(), 100);
        assert_eq!(layout.center_rows.iter().sum::<u16>(), 100);
        assert_eq!(layout.right_rows.iter().sum::<u16>(), 100);
        assert!(
            layout
                .vertical
                .iter()
                .all(|&weight| weight >= MIN_VERTICAL_WEIGHT)
        );
        assert!(
            layout
                .columns
                .iter()
                .all(|&weight| weight >= MIN_COLUMN_WEIGHT)
        );
        assert!(
            layout
                .left_rows
                .iter()
                .all(|&weight| weight >= MIN_STACK_WEIGHT)
        );
        assert!(
            layout
                .center_rows
                .iter()
                .all(|&weight| weight >= MIN_STACK_WEIGHT)
        );
        assert!(
            layout
                .right_rows
                .iter()
                .all(|&weight| weight >= MIN_STACK_WEIGHT)
        );
    }

    #[test]
    fn normalizes_extreme_layout_weights_without_overflow() {
        let layout = LayoutState {
            active_index: 0,
            vertical: [u16::MAX, u16::MAX],
            columns: [u16::MAX, u16::MAX, u16::MAX],
            left_rows: [u16::MAX, u16::MAX, u16::MAX],
            center_rows: [u16::MAX, u16::MAX],
            right_rows: [u16::MAX, u16::MAX],
            hidden: [false; 8],
        }
        .normalized();

        assert_eq!(layout.vertical.iter().sum::<u16>(), 100);
        assert_eq!(layout.columns.iter().sum::<u16>(), 100);
        assert_eq!(layout.left_rows.iter().sum::<u16>(), 100);
        assert_eq!(layout.center_rows.iter().sum::<u16>(), 100);
        assert_eq!(layout.right_rows.iter().sum::<u16>(), 100);
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
            hidden: [false, true, false, false, false, false, true, false],
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
        let raw = r#"{"active_index":4,"vertical":[84,16],"columns":[30,42,28],"left_rows":[45,30,25],"center_rows":[70,30],"right_rows":[55,45],"hidden":[false,true,false,false,false,false,true,false]}"#;
        let parsed = parse_local_layout(raw).unwrap();
        assert_eq!(parsed.active_index, 4);
        assert_eq!(parsed.vertical, [84, 16]);
        assert_eq!(parsed.columns, [30, 42, 28]);
        assert_eq!(parsed.left_rows, [45, 30, 25]);
        assert_eq!(parsed.center_rows, [70, 30]);
        assert_eq!(parsed.right_rows, [55, 45]);
        assert_eq!(
            parsed.hidden,
            [false, true, false, false, false, false, true, false]
        );
    }

    #[test]
    fn hidden_panes_roundtrip_through_layout_config() {
        let mut layout = LayoutState::default();
        layout.hide_pane(PaneId::Branches);
        layout.hide_pane(PaneId::Console);

        let rendered = toml::to_string(&LayoutConfig::from(layout.clone())).unwrap();
        let parsed: LayoutConfig = toml::from_str(&rendered).unwrap();
        let roundtrip = LayoutState::from(parsed);

        assert!(roundtrip.is_hidden(PaneId::Branches));
        assert!(roundtrip.is_hidden(PaneId::Console));
        assert!(!roundtrip.is_hidden(PaneId::Files));
    }

    #[test]
    fn hidden_console_expands_top_layout() {
        let mut layout = LayoutState::default();
        layout.hide_pane(PaneId::Console);

        let rects = layout.pane_rects(Rect::new(0, 0, 120, 40));
        let console = rects[PaneId::Console.index()];
        let files = rects[PaneId::Files.index()];
        let branches = rects[PaneId::Branches.index()];
        let stash = rects[PaneId::Stash.index()];

        assert_eq!(console.width, 0);
        assert_eq!(console.height, 0);
        assert_eq!(files.y, 0);
        assert_eq!(files.height + branches.height + stash.height, 40);
    }

    #[test]
    fn cannot_hide_last_visible_pane() {
        let mut layout = LayoutState::default();
        for pane in PaneId::ALL {
            if pane != PaneId::Files {
                assert!(layout.hide_pane(pane));
            }
        }

        assert!(!layout.hide_pane(PaneId::Files));
        assert!(!layout.is_hidden(PaneId::Files));
    }
}
