use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

use crate::normalize::normalize_field_names;

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct IssueCustomField {
    id: String,
    pub name: String,
    #[serde(rename = "$type")]
    pub r#type: String,
    pub value: serde_json::Value,
}

// pub enum IssueCustomFieldValue {
//     SingleEnumIssueCustomField { id: String, name: String },
//     SingleUserIssueCustomField { id: String, name: String },
//     StateIssueCustomField { id: String, name: String },
//     PeriodIssueCustomField { id: String },
//     MultiVersionIssueCustomField {},
//     MultiUserIssueCustomField {},
//     DateIssueCustomField {},
// }

impl IssueCustomField {
    pub fn fields() -> String {
        crate::normalize::normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY).replace(
            "value",
            &"value({id,name})"
        )
    }
}

