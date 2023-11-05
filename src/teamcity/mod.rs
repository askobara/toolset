pub mod build;
pub mod build_locator;
pub mod build_type;
pub mod build_type_locator;
pub mod deploy;
pub mod user;

use anyhow::{Context, Result};
use crate::teamcity::user::Triggered;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ArgBuildType {
    Build,
    Deploy,
    Any,
    Custom(String),
}

impl std::convert::From<&str> for ArgBuildType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "build" | "b" => ArgBuildType::Build,
            "deploy" | "d" => ArgBuildType::Deploy,
            "any" => ArgBuildType::Any,
            custom => ArgBuildType::Custom(custom.to_string()),
        }
    }
}

impl std::convert::From<ArgBuildType> for String {
    fn from(v: ArgBuildType) -> Self {
        match v {
            ArgBuildType::Build => "build".into(),
            ArgBuildType::Deploy => "deploy".into(),
            ArgBuildType::Any => "any".into(),
            ArgBuildType::Custom(custom) => custom,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildQueue {
    id: i32,
    build_type_id: String,
    state: String,
    branch_name: Option<String>,
    href: String,
    pub web_url: String,
    // build_type: BuildType,
    wait_reason: String,
    queued_date: String,
    triggered: Triggered,
}

#[derive(Debug, Deserialize)]
pub struct TeamcitySettings {
    pub client: crate::core::config::Config,
    pub build_types: HashMap<String, String>,
}

pub struct Client<'a> {
    pub http_client: crate::core::client::Client<'a>,
    config: &'a TeamcitySettings,
    pub build_type: Option<&'a str>,
    pub branch_name: Option<String>,
    pub repo: &'a crate::repo::Repo,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a TeamcitySettings, repo: &'a crate::repo::Repo) -> Result<Self> {
        let build_type = repo.get_name(None).and_then(|name| { Self::default_build_type(&name, config) }).ok();

        Ok(Self {
            http_client: crate::core::client::Client::new(&config.client)?,
            config,
            build_type,
            branch_name: repo.normalize_branch_name(None).ok(),
            repo,
        })
    }

    fn default_build_type(repo_name: &str, config: &'a TeamcitySettings) -> Result<&'a str> {
        config
            .build_types
            .get(repo_name)
            .map(|s| s.as_str())
            .context("No build type for current repo")
    }
}
