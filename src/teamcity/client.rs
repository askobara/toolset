use crate::normalize::*;
use crate::teamcity::config::TeamcitySettings;
use anyhow::{Context, Result};

pub struct Client<'a> {
    pub http_client: crate::core::client::Client<'a>,
    config: &'a TeamcitySettings,
    pub build_type: Option<&'a str>,
    pub branch_name: Option<String>,
    pub repo: &'a Repo,
}

impl<'a> Client<'a> {
    pub fn new(config: &'a TeamcitySettings, repo: &'a Repo) -> Result<Self> {
        let build_type = Self::default_build_type(repo, config).ok();

        Ok(Self {
            http_client: crate::core::client::Client::new(&config.client)?,
            config,
            build_type,
            branch_name: normalize_branch_name(None, repo).ok(),
            repo,
        })
    }

    fn default_build_type(repo: &Repo, config: &'a TeamcitySettings) -> Result<&'a str> {
        let file_name = get_repo_name(repo, None)?;

        config
            .build_types
            .get(&file_name)
            .map(|s| s.as_str())
            .context("No build type for current repo")
    }
}
