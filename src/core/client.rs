use anyhow::Result;
use reqwest::header;
use tracing::info;
use super::config::Config;
use colored_json::to_colored_json_auto;

pub struct Client<'a> {
    http_client: reqwest::Client,
    base_url: url::ParseOptions<'a>,
    config: &'a Config,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a Config) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .default_headers(Self::default_headers())
            .build()?;

        let base_url = {
            reqwest::Url::options().base_url(Some(&config.host()))
        };

        Ok(Self {
            http_client,
            base_url,
            config,
        })
    }

    fn default_headers() -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();

        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        headers
    }

    pub async fn get<U, R>(&self, url: U) -> Result<R>
    where
        U: Into<String>,
        R: serde::de::DeserializeOwned
    {
        let u = self.base_url.parse(&url.into()).map_err(anyhow::Error::new)?;

        info!("GET {u}");

        self
            .http_client
            .get(u)
            .bearer_auth(&self.config.auth_token())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(anyhow::Error::new)
    }

    pub async fn post<B, R, U>(&self, url: U, body: &B) -> Result<R>
    where
        U: Into<String>,
        B: serde::Serialize + std::fmt::Debug + ?Sized,
        R: serde::de::DeserializeOwned
    {
        let u = self.base_url.parse(&url.into()).map_err(anyhow::Error::new)?;

        #[cfg(windows)]
        let _enabled = colored_json::enable_ansi_support();

        info!("POST {u}\n{}", serde_json::to_value(&body).and_then(|v| to_colored_json_auto(&v))?);

        self
            .http_client
            .post(u)
            .bearer_auth(&self.config.auth_token())
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(anyhow::Error::new)
    }
}
