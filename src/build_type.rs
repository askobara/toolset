use anyhow::Result;
use crate::normalize::*;
use skim::prelude::*;
use crate::client::Client;
use serde::{Deserialize, Serialize};
use std::convert::AsRef;
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray, Clone)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct BuildType {
    pub(crate) id: String,
    name: String,
    web_url: String,
    r#type: Option<String>,
}

impl AsRef<str> for &BuildType {
    fn as_ref(&self) -> &str {
        self.id.as_str()
    }
}

impl SkimItem for BuildType {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.id)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("{:#?}", self))
    }
}

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct BuildTypes {
    count: i32,
    href: String,
    next_href: Option<String>,
    prev_href: Option<String>,
    pub(crate) build_type: Vec<BuildType>,
}

impl<'a> Client<'a> {
    pub async fn build_type_list(&self) -> Result<BuildTypes> {
        let fields = normalize_field_names(BuildTypes::FIELD_NAMES_AS_ARRAY).replace(
            "buildType",
            &format!("buildType({})", normalize_field_names(BuildType::FIELD_NAMES_AS_ARRAY))
        );

        let url = format!(
            // ?locator=type:regular,name:Build&
            "{host}/app/rest/buildTypes?fields={fields}",
            host = self.get_host(),
        );

        let response: BuildTypes = self.http_client.get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?
        ;

        Ok(response)
    }
}
