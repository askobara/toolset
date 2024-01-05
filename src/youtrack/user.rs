use anyhow::Result;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;
use super::Client;

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[field_names_as_array(rename_all = "camelCase")]
pub struct User {
    id: String,
    login: String,
}

impl User {
    pub fn fields() -> String {
        crate::normalize::normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY)
    }
}

impl<'a> Client<'a> {
    pub async fn get_user(&self, id: &str) -> Result<User> {
        let fields = User::fields();

        self.http_client.get(format!("/api/users/{id}?fields={fields}"))
            .await
    }

    pub async fn me(&self) -> Result<User> {
        self.get_user("me").await
    }
}
