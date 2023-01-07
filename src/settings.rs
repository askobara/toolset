use serde::Deserialize;
use config::Config;
use std::collections::HashMap;
use directories::ProjectDirs;
use std::fs;
use anyhow::{Result, Context};

#[derive(Debug, Deserialize)]
pub struct TeamcitySettings {
    pub host: String,
    pub auth_token: String,
    pub build_types: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub teamcity: TeamcitySettings,
}

impl Settings {
    pub fn new() -> Result<Self> {
        let config_path = ProjectDirs::from("", "", "teamcity").context("Couldn't retrive valid config folder")
            .and_then(|prj_dir| Ok(prj_dir.config_dir().join("config.toml")))?;

        if !config_path.exists() {
            fs::File::create(&config_path).expect("unable to create config file");
        }

        let settings = Config::builder()
            .add_source(config::File::with_name(config_path.to_str().context("Config path contains non-utf8 symbols")?))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;

        settings.try_deserialize().map_err(anyhow::Error::new)
    }
}
