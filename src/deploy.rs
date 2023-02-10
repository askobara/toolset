use serde::{Deserialize, Serialize};

use crate::build_locator::{BuildLocator, BuildLocatorBuilder};
use crate::build_type::BuildType;
use crate::client::Client;
use crate::normalize::select_one;
use crate::BuildQueue;
use anyhow::{bail, Context, Result};
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
    build_types: BuildTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectsWithBuildTypes {
    count: i32,
    project: Vec<ProjectWithBuildTypes>,
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
struct Build {
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

impl Build {
    fn build_types(&self) -> Vec<BuildType> {
        self.build_type
            .project
            .projects
            .project
            .iter()
            .flat_map(|prj| prj.build_types.build_type.iter().map(ToOwned::to_owned))
            .collect::<Vec<_>>()
    }
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
struct BuildTypeBody<'a> {
    id: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployBody<'a> {
    branch_name: Option<&'a str>,
    build_type: BuildTypeBody<'a>,
    #[serde(rename = "snapshot-dependencies")]
    snapshot_dependencies: DeployBuilds,
}

impl<'a> Client<'a> {
    async fn get_last_build(&self, locator: &BuildLocator<'_>) -> Result<Build> {
        let url = format!(
            "/app/rest/builds/{locator}?fields=id,buildTypeId,branchName,number,state,status,buildType:(id,name,project:(id,name,projects:(count,project:(id,name,buildTypes:(count,buildType)))))",
        );

        let build: Build = self.get(url).await?;

        match (build.state.as_str(), build.status.as_deref()) {
            (_, Some("FAILURE")) => bail!("Build #{id} is failed", id = build.id),
            ("queued", _) => bail!("Build #{id} is queued", id = build.id),
            (_, _) => Ok(build),
        }
    }

    pub async fn run_deploy(
        &self,
        build_id: Option<&str>,
        env: Option<&str>,
    ) -> Result<BuildQueue> {
        // TODO: deploy the last master build, when build_id is "master"
        // TODO: rerun deploy jobs

        let mut locator_builder = BuildLocatorBuilder::default();
        let id: Option<i32> = build_id.and_then(|v| v.parse().ok());

        if id.is_some() {
            locator_builder.id(id);
        } else {
            let btype = self.get_build_type_by_path().context("Current path doesn't have association with BuildType through config (or contains non-utf8 symbols)")?;

            locator_builder.build_type(Some(btype.to_string()));
            locator_builder.user(Some("current"));
        }

        let locator = locator_builder.build()?;
        let build = self.get_last_build(&locator).await?;

        info!("#{} {} {}", build.id, build.build_type_id, build.number);

        let selected_build_type = select_one(build.build_types(), env)?;

        let body = DeployBody {
            branch_name: build.branch_name.as_deref(),
            build_type: BuildTypeBody {
                id: &selected_build_type.id,
            },
            snapshot_dependencies: DeployBuilds {
                build: vec![DeployBuild { id: build.id }],
            },
        };

        let response: BuildQueue = self.post("/app/rest/buildQueue", &body).await?;

        Ok(response)
    }
}
