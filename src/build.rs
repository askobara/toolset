use crate::build_locator::BuildLocatorBuilder;
use crate::build_type_locator::BuildTypeLocator;
use crate::client::Client;
use crate::normalize::*;
use crate::user::{Triggered, User};
use crate::{ArgBuildType, BuildQueue};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use struct_field_names_as_array::FieldNamesAsArray;

#[derive(Debug, Serialize)]
struct BuildTypeBody<'a> {
    id: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildBody<'a> {
    branch_name: &'a str,
    build_type: BuildTypeBody<'a>,
}

#[derive(Debug, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct Build {
    pub(crate) id: i32,
    build_type_id: String,
    status: Option<String>, // SUCCESS/FAILURE/UNKNOWN
    state: String,          // queued/running/finished
    branch_name: Option<String>,
    web_url: String,
    finish_on_agent_date: Option<String>,
    triggered: Triggered,
}

fn format_datetime(datetime: &chrono::DateTime<chrono::FixedOffset>) -> String {
    let duration = chrono::Utc::now().signed_duration_since(*datetime);

    match (
        duration.num_hours(),
        duration.num_minutes(),
        duration.num_seconds(),
    ) {
        (12.., _, _) => datetime
            .with_timezone(&chrono::Local)
            .format("%a, %d %b %R")
            .to_string(),
        (hours @ 2..=12, _, _) => format!("{hours} hours ago"),
        (hours @ 1, _, _) => format!("{hours} hour ago"),
        (_, mins @ 2.., _) => format!("{mins} minutes ago"),
        (_, mins @ 1, _) => format!("{mins} minute ago"),
        (_, _, secs @ 10..) => format!("{secs} seconds ago"),
        (_, _, _) => "a few moments ago".to_string(),
    }
}

impl Build {
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn finished_at(&self) -> String {
        self.finish_on_agent_date
            .as_ref()
            .and_then(|str| chrono::DateTime::parse_from_str(str, "%Y%m%dT%H%M%S%z").ok())
            .map(|date| format_datetime(&date))
            .unwrap_or_default()
    }

    pub fn build_type_id(&self) -> &str {
        &self.build_type_id
    }

    pub fn web_url(&self) -> &str {
        &self.web_url
    }

    pub fn branch_name(&self) -> Option<&str> {
        self.branch_name.as_deref()
    }

    pub fn triggered_by(&self) -> &str {
        if let Some(user) = &self.triggered.user {
            return user.name.as_str();
        } else {
            return self.triggered.r#type.as_str();
        }
    }
}

#[derive(Debug, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct Builds {
    build: Vec<Build>,
}

impl IntoIterator for Builds {
    type Item = Build;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.build.into_iter()
    }
}

impl<'a> IntoIterator for &'a Builds {
    type Item = &'a Build;
    type IntoIter = std::slice::Iter<'a, Build>;

    fn into_iter(self) -> Self::IntoIter {
        self.build.iter()
    }
}

impl<'a> Client<'a> {
    pub async fn run_build(
        &self,
        build_type: Option<&str>,
        branch_name: Option<&str>,
    ) -> Result<BuildQueue> {
        let build_type = build_type
            .or_else(|| self.get_build_type_by_path().ok())
            .unwrap();
        // .context("Current path doesn't have association with BuildType through config (or contains non-utf8 symbols)")

        let branch = normalize_branch_name(branch_name, Some(&self.workdir))?;

        let body = BuildBody {
            build_type: BuildTypeBody { id: build_type },
            branch_name: &branch,
        };

        let response: BuildQueue = self.post("/app/rest/buildQueue", &body).await?;

        Ok(response)
    }

    pub async fn get_builds(
        &self,
        branch_name: Option<&str>,
        build_type: Option<&ArgBuildType>,
        author: Option<&str>,
        limit: Option<u8>,
    ) -> Result<Builds> {
        let branch = normalize_branch_name(branch_name, Some(&self.workdir))?;

        let locator = BuildLocatorBuilder::default()
            .count(limit)
            .user(author)
            .branch(Some(branch))
            .default_filter(Some(false))
            .personal(Some(false))
            .build_type(
                match build_type
                    .cloned()
                    .or_else(|| self.get_build_type_by_path().ok().map(Into::into))
                    .unwrap()
                {
                    ArgBuildType::Build => Some(BuildTypeLocator::only_builds()),
                    ArgBuildType::Deploy => Some(BuildTypeLocator::only_deploys()),
                    ArgBuildType::Custom(custom) => {
                        self.build_type_list()
                            .await
                            .and_then(|list| select_many(list.build_type, Some(&custom)))
                            .map(BuildTypeLocator::from)
                            .ok()
                    }
                    _ => None,
                },
            )
            .build()?;

        let fields = normalize_field_names(Builds::FIELD_NAMES_AS_ARRAY).replace(
            "build",
            &format!(
                "build({})",
                normalize_field_names(Build::FIELD_NAMES_AS_ARRAY).replace(
                    "triggered",
                    &format!(
                        "triggered({})",
                        normalize_field_names(Triggered::FIELD_NAMES_AS_ARRAY).replace(
                            "user",
                            &format!(
                                "user({})",
                                normalize_field_names(User::FIELD_NAMES_AS_ARRAY)
                            )
                        )
                    )
                )
            ),
        );

        let url = format!("/app/rest/builds?locator={locator}&fields={fields}");
        let response: Builds = self.get(url).await?;

        Ok(response)
    }
}
