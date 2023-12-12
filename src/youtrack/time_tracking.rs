use crate::youtrack::Client;

use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize)]
pub struct Duration {
    pub presentation: String,
}

#[derive(Debug, Serialize)]
pub struct Author {
    pub id: String
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeTracking {
    pub uses_markdown: bool,
    pub text: String,
    pub date: usize,
    pub author: Author,
    pub duration: Duration
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String
}

impl<'a> Client<'a> {
    pub async fn create_time_tracking(&self, id: &str, body: &TimeTracking) -> Result<WorkItem> {
        self.http_client.post(format!("/api/issues/{id}/timeTracking/workItems?fields=id"), body)
            .await
    }
}
