pub mod config;
pub mod build;
pub mod build_locator;
pub mod build_type;
pub mod build_type_locator;
pub mod client;
pub mod deploy;
pub mod user;

use crate::teamcity::user::Triggered;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ArgBuildType {
    Build,
    Deploy,
    Any,
    Custom(String),
}

impl std::convert::From<&str> for ArgBuildType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "build" | "b" => ArgBuildType::Build,
            "deploy" | "d" => ArgBuildType::Deploy,
            "any" => ArgBuildType::Any,
            custom => ArgBuildType::Custom(custom.to_string()),
        }
    }
}

impl std::convert::From<ArgBuildType> for String {
    fn from(v: ArgBuildType) -> Self {
        match v {
            ArgBuildType::Build => "build".into(),
            ArgBuildType::Deploy => "deploy".into(),
            ArgBuildType::Any => "any".into(),
            ArgBuildType::Custom(custom) => custom,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildQueue {
    id: i32,
    build_type_id: String,
    state: String,
    branch_name: Option<String>,
    href: String,
    pub web_url: String,
    // build_type: BuildType,
    wait_reason: String,
    queued_date: String,
    triggered: Triggered,
}
