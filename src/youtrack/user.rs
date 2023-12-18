use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[field_names_as_array(rename_all = "camelCase")]
pub struct User {
    id: String,
    login: String,
    email: String,
}

impl User {
    pub fn fields() -> String {
        crate::normalize::normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY)
    }
}
