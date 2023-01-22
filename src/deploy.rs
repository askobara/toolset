use serde::{Deserialize, Serialize};

use anyhow::{Result, Context, bail};
use std::fmt;
use skim::prelude::*;
use crate::{BuildType, BuildQueue};
use crate::client::Client;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildTypes {
    count: i32,
    build_type: Vec<BuildType>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWithBuildTypes {
    id: String,
    name: String,
    build_types: BuildTypes
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectsWithBuildTypes {
    count: i32,
    project: Vec<ProjectWithBuildTypes>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWithProjects {
    id: String,
    name: String,
    projects: ProjectsWithBuildTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildTypeWithProject {
    id: String,
    name: String,
    project: ProjectWithProjects,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    id: i32,
    build_type_id: String,
    branch_name: Option<String>,
    number: String,
    build_type: BuildTypeWithProject,
    /// queued/running/finished
    state: String,
    /// SUCCESS/FAILURE/UNKNOWN
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployBuild {
    id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployBuilds {
    build: Vec<DeployBuild>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BuildTypeBody {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployBody {
    branch_name: Option<String>,
    build_type: BuildTypeBody,
    #[serde(rename = "snapshot-dependencies")]
    snapshot_dependencies: DeployBuilds,
}

#[derive(Debug, Default)]
struct BuildLocator {
    id: Option<i32>,
    user: Option<String>,
    build_type: Option<String>,
}

impl BuildLocator {
    fn id(&mut self, value: Option<i32>) {
        self.id = value;
    }

    fn user(&mut self, value: Option<&str>) {
        self.user = value.map(ToOwned::to_owned);
    }

    fn build_type(&mut self, value: Option<&str>) {
        self.build_type = value.map(ToOwned::to_owned);
    }
}

impl fmt::Display for BuildLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut locators: Vec<String> = Vec::new();

        if let Some(id) = self.id {
            locators.push(format!("id:{}", id));
        }

        if let Some(user) = &self.user {
            locators.push(format!("user:{}", user));
        }

        if let Some(build_type) = &self.build_type {
            locators.push(format!("buildType:{}", build_type));
        }

        write!(f, "{}", locators.join(","))
    }
}

impl<'a> Client<'a> {
    async fn get_last_build(&self, locator: &BuildLocator) -> Result<Build> {
        let url = format!(
            "{host}/app/rest/builds/{locator}?fields=id,buildTypeId,branchName,number,state,status,buildType:(id,name,project:(id,name,projects:(count,project:(id,name,buildTypes:(count,buildType)))))",
            host = self.get_host(),
            locator = locator,
        );

        let build = self.http_client.get(url).send().await?.json::<Build>().await?;

        match (build.state.as_str(), build.status.as_deref()) {
            (_, Some("FAILURE")) => bail!("Build #{id} is failed", id = build.id),
            ("queued", _) => bail!("Build #{id} is queued", id = build.id),
            (_, _) => Ok(build),
        }
    }

    pub async fn run_deploy(
        &self,
        build_id: Option<&str>,
        env: Option<&str>
    ) -> Result<BuildQueue> {
        // TODO: deploy the last master build, when build_id is "master"

        let mut locator = BuildLocator::default();
        let id: Option<i32> = build_id.and_then(|v| v.parse().ok());

        if id.is_some() {
            locator.id(id);
        } else {
            let btype = self.get_build_type_by_path().context("Current path doesn't have association with BuildType through config (or contains non-utf8 symbols)")?;

            locator.build_type(Some(&btype));
            locator.user(Some("current"));
        }

        let build = self.get_last_build(&locator).await?;

        info!("#{} {} {}", build.id, build.build_type_id, build.number);

        let options = SkimOptionsBuilder::default()
            .prompt(Some("Select an environment where to deploy: "))
            .height(Some("30%"))
            .preview(Some(""))
            .preview_window(Some("right:70%"))
            .query(env)
            .select1(env.is_some())
            .build()
            .unwrap();

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

        build.build_type.project.projects.project.into_iter().for_each(|prj| {
            prj.build_types.build_type.into_iter().for_each(|bt| {
                let _ = tx_item.send(Arc::new(bt));
            });
        });
        drop(tx_item); // so that skim could know when to stop waiting for more items.

        let selected_items = Skim::run_with(&options, Some(rx_item))
            .filter(|out| !out.is_abort)
            .map(|out| out.selected_items)
            .unwrap_or_else(Vec::new);

        let selected_build_type: &BuildType = selected_items.first()
            .and_then(|v| (**v).as_any().downcast_ref())
            .context("No env selected")?;

        let body = DeployBody {
            branch_name: build.branch_name,
            build_type: BuildTypeBody {
                id: selected_build_type.id.clone(),
            },
            snapshot_dependencies: DeployBuilds {
                build: vec![
                    DeployBuild { id: build.id }
                ]
            }
        };

        let url = format!("{host}/app/rest/buildQueue", host = self.get_host());

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
}
