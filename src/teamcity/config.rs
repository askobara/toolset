use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TeamcitySettings {
    pub client: crate::core::config::Config,
    pub build_types: HashMap<String, String>,
}
