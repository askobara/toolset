use crate::youtrack::Client;

use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLink {
    id: String,
    name: String,
    target_to_source: String,
    source_to_target: String,
}

impl<'a> Client<'a> {
    pub async fn search_issue_link(&self, query: &str) -> Result<Vec<IssueLink>> {
        self.http_client.get(format!("/api/issueLinkTypes?fields=id,name,sourceToTarget,targetToSource&query={query}"))
            .await
    }
}


