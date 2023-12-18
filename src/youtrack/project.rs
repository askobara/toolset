use crate::youtrack::Client;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub short_name: String,
}

impl Project {
    pub fn fields() -> String {
        crate::normalize::normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY)
    }
}

impl<'a> Client<'a> {
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        self.http_client.get(format!("/api/admin/projects?fields=id,name,shortName"))
            .await
    }
}

