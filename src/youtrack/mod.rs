pub mod issue;
pub mod project;
pub mod comment;
pub mod time_tracking;
pub mod user;
pub mod custom_field;
pub mod tag;
pub mod issue_link;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct YoutrackConfig {
    pub client: crate::core::config::Config,
}

use anyhow::Result;

pub struct Client<'a> {
    pub http_client: crate::core::client::Client<'a>,
    config: &'a YoutrackConfig,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a YoutrackConfig) -> Result<Self> {
        Ok(Self {
            http_client: crate::core::client::Client::new(&config.client)?,
            config,
        })
    }
}
