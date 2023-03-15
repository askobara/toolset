use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
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
