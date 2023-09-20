use crate::youtrack::client::Client;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub short_name: String,
}

impl<'a> Client<'a> {
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        self.http_client.get(format!("/api/admin/projects?fields=id,name,shortName"))
            .await
    }
}

