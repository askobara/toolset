use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct YoutrackConfig {
    pub client: crate::core::config::Config,
}
