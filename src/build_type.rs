use anyhow::{Result, Context};
use crate::BuildType;
use crate::normalize::*;
use skim::prelude::*;
use crate::client::Client;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct BuildTypes {
    count: i32,
    href: String,
    next_href: Option<String>,
    prev_href: Option<String>,
    build_type: Vec<BuildType>,
}

impl<'a> Client<'a> {
    pub async fn build_type_list(&self) -> Result<BuildTypes> {
        let fields = normalize_field_names(BuildTypes::FIELD_NAMES_AS_ARRAY).replace(
            "buildType",
            &format!("buildType({})", normalize_field_names(BuildType::FIELD_NAMES_AS_ARRAY))
        );

        let url = format!(
            "{host}/app/rest/buildTypes?locator=type:regular,name:Build&fields={fields}",
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

    fn select_build_type(data: &Vec<BuildType>, query: Option<&str>) -> Result<BuildType> {
        let options = SkimOptionsBuilder::default()
            .height(Some("20%"))
            .query(query)
            .select1(query.is_some())
            .build()
            .unwrap()
        ;

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

        data.iter().for_each(|bt| {
            let _ = tx_item.send(Arc::new(bt.clone()));
        });
        drop(tx_item); // so that skim could know when to stop waiting for more items.

        let selected_items = Skim::run_with(&options, Some(rx_item))
            .filter(|out| !out.is_abort)
            .map(|out| out.selected_items)
            .unwrap_or_else(Vec::new);

        let selected_build_type: &BuildType = selected_items.first()
            .and_then(|v| (**v).as_any().downcast_ref())
            .context("No env selected")?;

        Ok(selected_build_type.to_owned())
    }
}
