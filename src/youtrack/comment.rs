use crate::youtrack::Client;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IssueComment {
    id: String,
    text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateIssueComment {
    text: String
}

impl<'a> Client<'a> {
    pub async fn comment_create(&self, id: &str, text: &str) -> Result<IssueComment> {

        let body = CreateIssueComment {
            text: text.to_string(),
        };

        self.http_client.post(format!("/api/issues/{id}/comments?fields=id,text"), &body)
            .await
    }
}
