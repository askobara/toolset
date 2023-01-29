use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::normalize::*;
use crate::{BuildQueue, ArgBuildType};
use crate::client::Client;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildTypeBody {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildBody {
    branch_name: String,
    build_type: BuildTypeBody,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub(crate) id: i32,
    build_type_id: String,
    number: Option<String>,
    status: Option<String>, // SUCCESS/FAILURE/UNKNOWN
    state: String, // queued/running/finished
    branch_name: Option<String>,
    href: String,
    web_url: String,
    finish_on_agent_date: Option<String>,
}

fn format_datetime(datetime: &chrono::DateTime<chrono::FixedOffset>) -> String {
    let duration = chrono::Utc::now().signed_duration_since(*datetime);

    match (duration.num_hours(), duration.num_minutes(), duration.num_seconds()) {
        (4 .., _, _) => datetime.with_timezone(&chrono::Local).format("%a, %d %b %R").to_string(),
        (hours @ 2 ..= 4, _, _) => format!("{hours} hours ago"),
        (hours @ 1, _, _) => format!("{hours} hour ago"),
        (_, mins @ 2 .., _) => format!("{mins} minutes ago"),
        (_, mins @ 1, _) => format!("{mins} minute ago"),
        (_, _, secs @ 10 ..) => format!("{secs} seconds ago"),
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
        self.finish_on_agent_date.as_ref()
            .and_then(|str| chrono::DateTime::parse_from_str(&str, "%Y%m%dT%H%M%S%z").ok())
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Builds {
    count: i32,
    href: String,
    next_href: Option<String>,
    prev_href: Option<String>,
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
    pub async fn run_build(&self, build_type: Option<&str>, branch_name: Option<&str>) -> Result<BuildQueue> {
        let build_type = build_type.map(|s| s.to_string()).or_else(|| self.get_build_type_by_path().ok()).unwrap();
        // .context("Current path doesn't have association with BuildType through config (or contains non-utf8 symbols)")

        let branch = normalize_branch_name(branch_name, Some(&self.workdir))?;

        let body = BuildBody {
            build_type: BuildTypeBody {
                id: build_type.into(),
            },
            branch_name: branch.clone(),
        };

        let url = format!("{}/app/rest/buildQueue", self.get_host());

        let response: BuildQueue = self.http_client.post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?
        ;

        Ok(response)
    }

    pub async fn get_builds(
        &self,
        branch_name: Option<&str>,
        build_type: Option<&ArgBuildType>,
        author: Option<&str>,
        limit: Option<u8>
    ) -> Result<Builds> {
        let branch = normalize_branch_name(branch_name, Some(&self.workdir))?;

        let mut locator: Vec<String> = vec![
            format!("defaultFilter:false"),
            format!("personal:false"),
            format!("count:{}", limit.unwrap_or(5))
        ];

        if branch != "any" {
            locator.push(format!("branch:{branch}"));
        } else {
            locator.push("branch:default:any".to_string());
        }

        match build_type.cloned().or_else(|| self.get_build_type_by_path().ok().map(|p| ArgBuildType::from(p.as_str()))).unwrap() {
            ArgBuildType::Build => locator.push("buildType:(type:regular,name:Build)".to_string()),
            ArgBuildType::Deploy => locator.push("buildType:(type:deployment)".to_string()),
            ArgBuildType::Custom(custom) => {
                let bt = self.build_type_list().await.and_then(|list| {
                    select_one(list.build_type, Some(&custom))
                })?;
                locator.push(format!("buildType:{name}", name = bt.id))
            },
            _ => {},
        };

        if let Some(author) = author {
            // let user_list = self.user_list().await?;
            // let user = select(&user_list.user, Some(&author))?;
            // locator.push(format!("user:{name}", name = user.username));
            locator.push(format!("user:{author}"));
        }

        let url = format!(
            "{host}/app/rest/builds?locator={locator}",
            host = self.get_host(),
            locator = locator.join(",")
        );

        info!("{}", &url);

        let response: Builds = self.http_client.get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?
        ;

        Ok(response)
    }
}
