use serde::{Deserialize, Serialize};

use skim::prelude::*;
use crate::{BuildType, BuildQueue, BuildTypeBody, CONFIG};

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
    branch_name: String,
    number: String,
    build_type: BuildTypeWithProject,
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
#[serde(rename_all = "camelCase")]
struct DeployBody {
    branch_name: String,
    build_type: BuildTypeBody,
    #[serde(rename = "snapshot-dependencies")]
    snapshot_dependencies: DeployBuilds,
}

pub async fn run_deploy(client: &reqwest::Client, build_id: &str, env: Option<&str>) -> Result<BuildQueue, Box<dyn std::error::Error>>
{
    let build = client.get(format!("{host}/app/rest/builds/id:{build_id}?fields=id,buildTypeId,branchName,number,buildType:(id,name,project:(id,name,projects:(count,project:(id,name,buildTypes:(count,buildType)))))", host = CONFIG.teamcity.host))
        .send()
        .await?
        .json::<Build>()
        .await?
    ;

    let options = SkimOptionsBuilder::default()
        .header(Some("Select build env:"))
        .reverse(true)
        .query(env)
        .height(Some("30%"))
        .multi(false)
        .preview(Some(""))
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    build.build_type.project.projects.project.into_iter().for_each(|prj| {
        prj.build_types.build_type.into_iter().for_each(|bt| {
            let _ = tx_item.send(Arc::new(bt));
        });
    });
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_environments = Skim::run_with(&options, Some(rx_item))
        .map(|out| {
            if !out.is_abort {
                out.selected_items
            } else {
                Vec::new()
            }
        })
        .unwrap_or_else(Vec::new);

    let selected_build_type = selected_environments.first().expect("No env selected");

    let body = DeployBody {
        branch_name: build.branch_name,
        build_type: BuildTypeBody {
            id: selected_build_type.text().to_string(),
        },
        snapshot_dependencies: DeployBuilds {
            build: vec![
                DeployBuild { id: build.id }
            ]
        }
    };

    let response = client.post(format!("{host}/app/rest/buildQueue", host = CONFIG.teamcity.host))
        .json(&body)
        .send()
        .await?
        .json::<BuildQueue>()
        .await?
    ;

    Ok(response)
}
