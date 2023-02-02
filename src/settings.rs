use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamcitySettings {
    pub host: String,
    pub auth_token: String,
    pub build_types: HashMap<String, String>,
}

impl Default for TeamcitySettings {
    fn default() -> Self {
        Self {
            host: "".to_string(),
            auth_token: "".to_string(),
            build_types: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub teamcity: TeamcitySettings,
}

impl Settings {
    pub fn new() -> Result<Self> {
        Self::config_path()
            .and_then(|path| File::open(path).map_err(anyhow::Error::new))
            .and_then(|file| serde_yaml::from_reader(file).map_err(anyhow::Error::new))
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_path = ProjectDirs::from("", "", "teamcity")
            .context("Couldn't retrive project dirs")
            .map(|prj_dirs| prj_dirs.config_dir().join("config.yaml"))?;

        if !config_path.exists() {
            File::create(&config_path)?;
        }

        Ok(config_path)
    }
}
