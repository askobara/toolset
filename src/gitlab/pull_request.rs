use crate::repo::BranchNameMeta;

use super::{Client, project::Project};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;
use derive_more::Display;

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub title: String,
    pub web_url: Url,
    pub blocking_discussions_resolved: bool,
    pub user_notes_count: u32,
    pub has_conflicts: bool,
}

#[derive(Display)]
pub enum State {
    #[display(fmt="all")]
    All,

    #[display(fmt="opened")]
    Opened,
}

#[derive(Debug, Serialize)]
pub struct CreatePullRequestBody {
    source_branch: String,
    target_branch: String,
    title: String,
    squash_on_merge: bool,
    remove_source_branch: bool,
}

impl<'a> Client<'a> {
    pub async fn get_pull_requests(&self, branch: &str, state: State) -> Result<Vec<PullRequest>> {
        let url = format!(
            "/api/v4/merge_requests?source_branch={branch}&state={state}",
        );

        let pull_requests: Vec<PullRequest> = self.http_client.get(url).await?;

        Ok(pull_requests)
    }

    pub async fn create_pull_request(&self, prj: &Project, bn: &BranchNameMeta) -> Result<PullRequest> {
        let url = format!(
            "/api/v4/projects/{}/merge_requests", prj.id,
        );

        let body = CreatePullRequestBody {
            source_branch: bn.upstream_name.clone().unwrap(),
            target_branch: "master".to_owned(),
            title: bn.summary.clone().unwrap(),
            squash_on_merge: true,
            remove_source_branch: true,
        };

        let response: PullRequest = self.http_client.post(url, &body).await?;

        Ok(response)
    }
}
