use crate::youtrack::config::YoutrackConfig;
use crate::normalize::*;
use anyhow::{Context, Result};
use reqwest::header;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct Client<'a> {
    pub(crate) http_client: reqwest::Client,
    pub(crate) workdir: PathBuf,
    config: &'a YoutrackConfig,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a YoutrackConfig, workdir: Option<&Path>) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .default_headers(Self::default_headers(config)?)
            .build()?;

        Ok(Self {
            http_client,
            config,
            workdir: normalize_path(workdir)?,
        })
    }

    fn default_headers(config: &YoutrackConfig) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();

        let token = format!("Bearer {}", config.auth_token.clone());
        // Consider marking security-sensitive headers with `set_sensitive`.
        let mut auth_value = header::HeaderValue::from_str(&token)?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        Ok(headers)
    }

    pub async fn get<U, R>(&self, url: U) -> Result<R>
    where
        U: Into<String>,
        R: serde::de::DeserializeOwned
    {
        let u = reqwest::Url::parse(&self.config.host)
            .and_then(|u| u.join(&url.into()))
            .map_err(anyhow::Error::new)?;

        debug!("{u}");

        self
            .http_client
            .get(u)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(anyhow::Error::new)
    }

}
