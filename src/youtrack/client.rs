use super::config::YoutrackConfig;
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
