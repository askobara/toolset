use super::Client;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: u32,
    pub name: String,
    pub name_with_namespace: String,
}

impl<'a> Client<'a> {
    pub async fn find_project_by_name(&self, name: &str) -> Result<Vec<Project>> {
        self.http_client.get(format!("/api/v4/projects?search={name}"))
            .await
    }
}
