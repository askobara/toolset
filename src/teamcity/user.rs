use crate::teamcity::Client;
use crate::normalize::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use skim::prelude::*;
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Serialize, Deserialize, Clone, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(visibility = "pub")]
pub struct User {
    pub(crate) username: String,
    pub(crate) name: String,
    id: i32,
}

impl SkimItem for User {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.username)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("{self:#?}"))
    }
}

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(visibility = "pub")]
pub struct Triggered {
    pub r#type: String,
    date: String,
    pub user: Option<User>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
pub struct Users {
    count: i32,
    pub(crate) user: Vec<User>,
}

impl Users {
    pub fn fields() -> String {
        normalize_field_names(&Users::FIELD_NAMES_AS_ARRAY).replace(
            "user",
            &format!(
                "user({})",
                normalize_field_names(&User::FIELD_NAMES_AS_ARRAY)
            ),
        )
    }
}

impl<'a> Client<'a> {
    pub async fn user_list(&self) -> Result<Users> {
        let fields = Users::fields();
        let url = format!("/app/rest/users?fields={fields}");
        let response: Users = self.http_client.get(url).await?;

        Ok(response)
    }
}
