use anyhow::Result;
use crate::client::Client;
use crate::normalize::*;
use skim::prelude::*;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Serialize, Deserialize, Clone, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub(crate) username: String,
    name: String,
    id: i32,
    email: String,
}

impl SkimItem for User {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.username)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("{:#?}", self))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
pub struct Users {
    count: i32,
    pub(crate) user: Vec<User>,
}

impl<'a> Client<'a> {
    pub async fn user_list(&self) -> Result<Users> {
        let fields = normalize_field_names(Users::FIELD_NAMES_AS_ARRAY).replace(
            "user",
            &format!("user({})", normalize_field_names(User::FIELD_NAMES_AS_ARRAY))
        );

        let url = format!(
            "{host}/app/rest/users?fields={fields}",
            host = self.get_host(),
        );

        let response: Users = self.http_client.get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?
        ;

        Ok(response)
    }

}
