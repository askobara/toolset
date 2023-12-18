use crate::youtrack::Client;

use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
}

impl<'a> Client<'a> {
    pub async fn search_tags(&self, query: &str) -> Result<Vec<Tag>> {
        self.http_client.get(format!("/api/tags?fields=id,name&query={query}"))
            .await
    }
}

