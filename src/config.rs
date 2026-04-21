use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CogitConfig {
    pub editor: String,
    pub theme: String,
    pub keymap_overrides: HashMap<String, String>,
}

impl CogitConfig {
    pub fn default() -> Self {
        Self {
            editor: "vim".to_string(),
            theme: "default".to_string(),
            keymap_overrides: HashMap::new(),
        }
    }

    pub fn load() -> anyhow::Result<Self> {
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "cogit") {
            let config_path = config_dir.config_dir().join("config.toml");
            if config_path.exists() {
                let settings = config::Config::builder()
                    .add_source(config::File::from(config_path))
                    .build()?;
                let cfg: CogitConfig = settings.try_deserialize()?;
                return Ok(cfg);
            }
        }
        Ok(Self::default())
    }
}
