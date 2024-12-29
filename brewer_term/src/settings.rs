use std::path::PathBuf;
use std::time::Duration;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AutoUpdate {
    Never,

    #[serde(untagged)]
    Every(Duration),
}

impl Default for AutoUpdate {
    fn default() -> Self {
        AutoUpdate::Every(Duration::from_secs(60 * 60 * 24))
    }
}

#[derive(Deserialize, Default)]
pub struct Cache {
    #[serde(default)]
    pub auto_update: AutoUpdate,
}

#[derive(Deserialize, Default)]
pub struct Homebrew {
    pub path: Option<PathBuf>,
    pub prefix: Option<PathBuf>,
}

#[derive(Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub homebrew: Homebrew,

    #[serde(default)]
    pub cache: Cache,
}

impl Settings {
    fn config_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        let base = dirs::home_dir().map(|p| p.join(".config"));

        #[cfg(not(target_os = "macos"))]
        let base = dirs::config_local_dir();

        base.map(|p| p.join("brewer")).unwrap_or(".".into())
    }

    pub fn config_file() -> PathBuf {
        Self::config_dir().join("brewer")
    }

    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name(Self::config_file().to_str().unwrap()).required(false))
            .add_source(Environment::with_prefix("brewer"))
            .build()?;

        settings.try_deserialize()
    }
}

