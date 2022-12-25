use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::normalize::*;
use crate::{BuildQueue, Builds, CONFIG};

#[derive(Debug, Serialize, Deserialize)]
struct BuildTypeBody {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildBody {
    branch_name: String,
    build_type: BuildTypeBody,
}

pub async fn run_build(client: &reqwest::Client, workdir: Option<&str>, branch_name: Option<&str>) -> Result<BuildQueue> {
    let path = normalize_path(workdir);
    let branch = normalize_branch_name(branch_name, &path);
    let build_type = get_build_type_by_path(&path);

    let body = BuildBody {
        build_type: BuildTypeBody {
            id: build_type.clone(),
        },
        branch_name: branch.clone(),
    };

    let response = client.post(format!("{}/app/rest/buildQueue", CONFIG.teamcity.host))
        .json(&body)
        .send()
        .await?
        .json::<BuildQueue>()
        .await?;

    Ok(response)
}

pub async fn get_builds(client: &reqwest::Client, workdir: Option<&str>, branch_name: Option<&str>, build_type: Option<&str>, author: Option<&str>, limit: Option<u8>) -> Result<Builds> {
    let path = normalize_path(workdir);
    let branch = normalize_branch_name(branch_name, &path);
    let btype = normalize_build_type(build_type, &path);

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

    if btype == "build" || btype == "b" {
        locator.push("buildType:(type:regular,name:Build)".to_string());
    } else if btype == "deploy" || btype == "d" {
        locator.push("buildType:(type:deployment)".to_string());
    } else if btype != "any" {
        locator.push(format!("buildType:{btype}"));
    }

    if let Some(author) = author {
        locator.push(format!("user:{author}"));
    }

    let url = format!(
        "{host}/app/rest/builds?locator={locator}",
        host = CONFIG.teamcity.host,
        locator = locator.join(",")
    );

    info!("{}", &url);

    let response = client.get(url)
        .send()
        .await?
        .json::<Builds>()
        .await?
    ;

    Ok(response)
}
