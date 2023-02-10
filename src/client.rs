use crate::normalize::*;
use crate::settings::TeamcitySettings;
use anyhow::{Context, Result};
use reqwest::header;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct Client<'a> {
    pub(crate) http_client: reqwest::Client,
    pub(crate) workdir: PathBuf,
    settings: &'a TeamcitySettings,
}

impl<'a> Client<'a> {
    pub fn new(settings: &'a TeamcitySettings, workdir: Option<&Path>) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .default_headers(Self::default_headers(settings)?)
            .build()?;

        Ok(Self {
            http_client,
            settings,
            workdir: normalize_path(workdir)?,
        })
    }

    pub async fn post<B, R>(&self, url: &str, body: &B) -> Result<R>
    where
        B: serde::Serialize + ?Sized,
        R: serde::de::DeserializeOwned
    {
        let u = reqwest::Url::parse(&self.settings.host)
            .and_then(|u| u.join(url))
            .map_err(anyhow::Error::new)?;

        info!("{u}");

        self
            .http_client
            .post(u)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(anyhow::Error::new)
    }

    pub async fn get<U, R>(&self, url: U) -> Result<R>
    where
        U: Into<String>,
        R: serde::de::DeserializeOwned
    {
        let u = reqwest::Url::parse(&self.settings.host)
            .and_then(|u| u.join(&url.into()))
            .map_err(anyhow::Error::new)?;

        info!("{u}");

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

    fn default_headers(settings: &TeamcitySettings) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();

        let token = format!("Bearer {}", settings.auth_token.clone());
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

    pub fn get_build_type_by_path(&self) -> Result<&str> {
        let repo = git2::Repository::discover(&self.workdir)?;
        let remote = repo.find_remote("origin")?;
        let url = remote.url().context("No url for origin")?;
        let file_name = Path::new(url)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_owned())
            .context("Cannot get repo name")?;

        // if !self.build_types.contains_key(&file_name) {
        //     let bt = self.build_type_list().await?;
        //     let r = crate::build_type::select_build_type(&bt.build_type, None)?;
        //     self.build_types.insert(file_name.clone(), r.id);
        // }

        self.settings
            .build_types
            .get(&file_name)
            .map(|s| s.as_str())
            .context("No build type for current repo")
    }
}
