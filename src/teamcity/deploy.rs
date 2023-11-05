use serde::{Deserialize, Serialize};

use crate::teamcity::build_locator::{BuildLocator, BuildLocatorBuilder};
use crate::teamcity::Client;
use crate::normalize::select_one;
use crate::teamcity::BuildQueue;
use anyhow::{bail, Result};
use tracing::debug;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Build {
    id: i32,
    build_type_id: String,
    branch_name: Option<String>,
    number: String,
    /// queued/running/finished
    state: String,
    /// SUCCESS/FAILURE/UNKNOWN
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeployBuild {
    id: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeployBuilds {
    build: Vec<DeployBuild>,
}

#[derive(Debug, Serialize)]
struct BuildTypeBody<'a> {
    id: &'a str,
}

#[derive(Debug, Serialize)]
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
            "/app/rest/builds/{locator}?fields=id,buildTypeId,branchName,number,state,status",
        );

        let build: Build = self.http_client.get(url).await?;

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
        branch_name: Option<&str>,
    ) -> Result<BuildQueue> {
        // TODO: deploy the last master build, when build_id is "master"
        // TODO: rerun deploy jobs

        let mut locator_builder = BuildLocatorBuilder::default();
        let id: Option<i32> = build_id.and_then(|v| v.parse().ok());

        if id.is_some() {
            locator_builder.id(id);
        } else {
            let branch = self.repo.normalize_branch_name(branch_name)?;

            locator_builder.build_type(self.build_type);
            locator_builder.branch(Some(branch));

            if branch_name.is_none() {
                locator_builder.user(Some("current"));
            }
        }

        let locator = locator_builder.build()?;
        let build = self.get_last_build(&locator).await?;

        debug!("#{} {} {}", build.id, build.build_type_id, build.number);
        let deploments = self.deployment_list(&build.build_type_id).await?;

        let selected_build_type = select_one(deploments.build_type, env)?;

        let body = DeployBody {
            branch_name: build.branch_name.as_deref(),
            build_type: BuildTypeBody {
                id: &selected_build_type.id,
            },
            snapshot_dependencies: DeployBuilds {
                build: vec![DeployBuild { id: build.id }],
            },
        };

        let response: BuildQueue = self.http_client.post("/app/rest/buildQueue", &body).await?;

        Ok(response)
    }
}
