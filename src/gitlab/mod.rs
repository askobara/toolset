pub mod pull_request;
pub mod project;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitlabConfig {
    client: crate::core::config::Config,
}

pub struct Client<'a> {
    pub http_client: crate::core::client::Client<'a>,
    config: &'a GitlabConfig,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a GitlabConfig) -> Result<Self> {
        Ok(Self {
            http_client: crate::core::client::Client::new(&config.client)?,
            config,
        })
    }
}
