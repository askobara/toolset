use serde::{Deserialize, Serialize};
use crate::normalize::*;
use crate::{BuildQueue, CONFIG, save_as_last_build};

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

pub async fn run_build(client: &reqwest::Client, workdir: Option<&str>, branch_name: Option<&str>) -> Result<BuildQueue, Box<dyn std::error::Error>> {
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

    println!("{}", response.web_url);

    save_as_last_build(&response);

    Ok(response)
}
