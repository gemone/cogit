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
}

impl Default for CogitConfig {
    fn default() -> Self {
        Self {
            keymap: KeymapConfig::default(),
        }
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
        let rendered = toml::to_string_pretty(&self.config)
            .context("failed to serialize cogit config")?;
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
}
