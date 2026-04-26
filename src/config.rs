use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CogitConfig {
    #[serde(default)]
    pub keymap: KeymapConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
}

impl Default for CogitConfig {
    fn default() -> Self {
        Self {
            keymap: KeymapConfig::default(),
            layout: LayoutConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "LayoutConfigCompat")]
pub struct LayoutConfig {
    pub active_index: usize,
    pub vertical: [u16; 2],
    pub columns: [u16; 3],
    pub left_rows: [u16; 3],
    pub center_rows: [u16; 2],
    pub right_rows: [u16; 2],
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            active_index: default_layout_active_index(),
            vertical: default_layout_vertical(),
            columns: default_layout_columns(),
            left_rows: default_layout_left_rows(),
            center_rows: default_layout_center_rows(),
            right_rows: default_layout_right_rows(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LayoutConfigCompat {
    New(LayoutConfigV2),
    Legacy(LayoutConfigLegacy),
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutConfigV2 {
    #[serde(default = "default_layout_active_index")]
    active_index: usize,
    #[serde(default = "default_layout_vertical")]
    vertical: [u16; 2],
    #[serde(default = "default_layout_columns")]
    columns: [u16; 3],
    #[serde(default = "default_layout_left_rows")]
    left_rows: [u16; 3],
    #[serde(default = "default_layout_center_rows")]
    center_rows: [u16; 2],
    #[serde(default = "default_layout_right_rows")]
    right_rows: [u16; 2],
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutConfigLegacy {
    #[serde(default = "default_layout_active_index")]
    active_index: usize,
    #[serde(default = "default_legacy_layout_columns")]
    columns: [u16; 4],
    #[serde(default = "default_legacy_layout_rows")]
    rows: [u16; 2],
}

impl From<LayoutConfigCompat> for LayoutConfig {
    fn from(value: LayoutConfigCompat) -> Self {
        match value {
            LayoutConfigCompat::New(v2) => Self {
                active_index: v2.active_index,
                vertical: v2.vertical,
                columns: v2.columns,
                left_rows: v2.left_rows,
                center_rows: v2.center_rows,
                right_rows: v2.right_rows,
            },
            LayoutConfigCompat::Legacy(legacy) => {
                let mut layout = LayoutConfig::default();
                layout.active_index = map_legacy_active_index(legacy.active_index);

                let left = legacy.columns[0].max(20);
                let center = legacy.columns[1].saturating_add(legacy.columns[2]).max(36);
                let right = legacy.columns[3].max(20);
                let total = left + center + right;
                layout.columns = [
                    ((left as u32 * 100) / total as u32) as u16,
                    ((center as u32 * 100) / total as u32) as u16,
                    100u16
                        .saturating_sub(((left as u32 * 100) / total as u32) as u16)
                        .saturating_sub(((center as u32 * 100) / total as u32) as u16),
                ];

                let console_height =
                    ((legacy.rows[1] as u32 * legacy.columns[0] as u32) / 100) as u16;
                let bottom = console_height.clamp(12, 35);
                layout.vertical = [100 - bottom, bottom];
                layout
            }
        }
    }
}

pub(crate) fn default_layout_active_index() -> usize {
    0
}

pub(crate) fn default_layout_vertical() -> [u16; 2] {
    [82, 18]
}

pub(crate) fn default_layout_columns() -> [u16; 3] {
    [28, 44, 28]
}

pub(crate) fn default_layout_left_rows() -> [u16; 3] {
    [48, 28, 24]
}

pub(crate) fn default_layout_center_rows() -> [u16; 2] {
    [68, 32]
}

pub(crate) fn default_layout_right_rows() -> [u16; 2] {
    [52, 48]
}

fn default_legacy_layout_columns() -> [u16; 4] {
    [28, 24, 24, 24]
}

fn default_legacy_layout_rows() -> [u16; 2] {
    [62, 38]
}

fn map_legacy_active_index(index: usize) -> usize {
    match index {
        0 => 0, // Files
        1 => 1, // Branches
        2 => 3, // Log
        3 => 4, // Rebase
        4 => 7, // Console
        5 => 2, // Stash
        6 => 5, // Remote
        7 => 6, // Shelve
        _ => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapConfig {
    #[serde(default)]
    pub preset: KeymapPreset,
    #[serde(default)]
    pub overrides: KeymapOverrides,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            preset: KeymapPreset::Vim,
            overrides: KeymapOverrides::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeymapOverrides {
    #[serde(default)]
    pub global: BTreeMap<String, String>,
    #[serde(default)]
    pub views: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeymapPreset {
    #[default]
    Vim,
    Helix,
}

impl KeymapPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            KeymapPreset::Vim => "vim",
            KeymapPreset::Helix => "helix",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub config: CogitConfig,
}

impl ConfigFile {
    pub fn load() -> Result<Self> {
        let path = config_path().context("failed to resolve cogit config path")?;
        let config = if path.exists() {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read config file: {}", path.display()))?;
            toml::from_str(&raw)
                .with_context(|| format!("failed to parse config file: {}", path.display()))?
        } else {
            CogitConfig::default()
        };
        Ok(Self { path, config })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir: {}", parent.display()))?;
        }
        let rendered =
            toml::to_string_pretty(&self.config).context("failed to serialize cogit config")?;
        fs::write(&self.path, rendered)
            .with_context(|| format!("failed to write config file: {}", self.path.display()))?;
        Ok(())
    }

    pub fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("one", "gemo", "cogit").map(|dirs| dirs.config_dir().to_path_buf())
    }
}

pub fn config_path() -> Option<PathBuf> {
    ConfigFile::config_dir().map(|dir| dir.join("config.toml"))
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keymap_preset_from_toml() {
        let cfg: CogitConfig = toml::from_str("[keymap]\npreset = 'helix'\n").unwrap();
        assert_eq!(cfg.keymap.preset, KeymapPreset::Helix);
    }

    #[test]
    fn defaults_to_vim_preset() {
        let cfg = CogitConfig::default();
        assert_eq!(cfg.keymap.preset, KeymapPreset::Vim);
    }

    #[test]
    fn parses_new_layout_config_from_toml() {
        let cfg: CogitConfig = toml::from_str(
            "[layout]\nactive_index = 4\nvertical = [84, 16]\ncolumns = [30, 42, 28]\nleft_rows = [45, 30, 25]\ncenter_rows = [70, 30]\nright_rows = [55, 45]\n",
        )
        .unwrap();
        assert_eq!(cfg.layout.active_index, 4);
        assert_eq!(cfg.layout.vertical, [84, 16]);
        assert_eq!(cfg.layout.columns, [30, 42, 28]);
        assert_eq!(cfg.layout.left_rows, [45, 30, 25]);
        assert_eq!(cfg.layout.center_rows, [70, 30]);
        assert_eq!(cfg.layout.right_rows, [55, 45]);
    }

    #[test]
    fn parses_legacy_layout_config_from_toml() {
        let cfg: CogitConfig = toml::from_str(
            "[layout]\nactive_index = 5\ncolumns = [28, 24, 24, 24]\nrows = [62, 38]\n",
        )
        .unwrap();
        assert_eq!(cfg.layout.active_index, 2);
        assert_eq!(cfg.layout.vertical, [88, 12]);
        assert_eq!(cfg.layout.columns, [28, 48, 24]);
    }
}
