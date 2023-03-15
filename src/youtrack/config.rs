use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct YoutrackConfig {
    pub host: String,
    pub auth_token: String,
}

