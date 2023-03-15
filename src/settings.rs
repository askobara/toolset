use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub teamcity: crate::teamcity::config::TeamcitySettings,
    pub youtrack: crate::youtrack::config::YoutrackConfig,
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
