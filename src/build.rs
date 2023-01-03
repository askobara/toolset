use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use crate::normalize::*;
use crate::{BuildQueue, Builds, ArgBuildType, CONFIG};
use tracing::info;

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

pub async fn run_build(client: &reqwest::Client, workdir: Option<&std::path::Path>, branch_name: Option<&str>) -> Result<BuildQueue> {
    let path = normalize_path(workdir)?;
    let branch = normalize_branch_name(branch_name, workdir)?;
    let build_type = get_build_type_by_path(&path).context("Current path doesn't have association with BuildType through config (or contains non-utf8 symbols)")?;

    let body = BuildBody {
        build_type: BuildTypeBody {
            id: build_type.into(),
        },
        branch_name: branch.clone(),
    };

    let response: BuildQueue = client.post(format!("{}/app/rest/buildQueue", CONFIG.teamcity.host))
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
    client: &reqwest::Client,
    workdir: Option<&std::path::Path>,
    branch_name: Option<&str>,
    build_type: Option<&ArgBuildType>,
    author: Option<&str>,
    limit: Option<u8>
) -> Result<Builds> {
    let path = normalize_path(workdir)?;
    let branch = normalize_branch_name(branch_name, workdir)?;

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

    match build_type.cloned().or_else(|| get_build_type_by_path(&path).map(|p| ArgBuildType::from(p.as_str()))).unwrap() {
        ArgBuildType::Build => locator.push("buildType:(type:regular,name:Build)".to_string()),
        ArgBuildType::Deploy => locator.push("buildType:(type:deployment)".to_string()),
        ArgBuildType::Custom(custom) => locator.push(format!("buildType:{custom}")),
        _ => {},
    };

    if let Some(author) = author {
        locator.push(format!("user:{author}"));
    }

    let url = format!(
        "{host}/app/rest/builds?locator={locator}",
        host = CONFIG.teamcity.host,
        locator = locator.join(",")
    );

    info!("{}", &url);

    let response: Builds = client.get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?
    ;

    Ok(response)
}
