use std::borrow::Cow;
use url::Url;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: Url,
    pub auth_token: String,
}

impl Config {
    pub fn host(&self) -> &Url {
        &self.host
    }

    pub fn auth_token(&self) -> Cow<str> {
        Cow::Borrowed(&self.auth_token)
    }
}
