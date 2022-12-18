use serde::Deserialize;
use config::{Config, ConfigError};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct TeamcitySettings {
    pub host: String,
    pub auth_token: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub teamcity: TeamcitySettings,
    pub build_types: HashMap<String, String>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config_path = xdg::BaseDirectories::with_prefix("teamcity").ok()
            .and_then(|xdg_dir| xdg_dir.place_config_file("config.toml").ok())
            .and_then(|path| {
                if !path.as_path().exists() {
                    fs::File::create(&path).expect("unable to create config file");
                }
                Some(path)
            })
            .unwrap();

        let settings = Config::builder()
            .add_source(config::File::with_name(config_path.to_str().unwrap()))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(config::Environment::with_prefix("APP"))
            .build()
            .unwrap();

        settings.try_deserialize()
    }
}
