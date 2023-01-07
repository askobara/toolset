use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use crate::CONFIG;

pub fn normalize_path(path: Option<&Path>) -> std::io::Result<PathBuf> {
    match path {
        Some(p) => p.canonicalize(),
        None => std::env::current_dir()
    }
}

pub fn normalize_branch_name(branch_name: Option<&str>, path: Option<&Path>) -> Result<String> {
    match branch_name {
        Some(bn) => Ok(bn.into()),
        None => {
            let p = normalize_path(path)?;
            let repo = git2::Repository::discover(p)?;
            let head = repo.head()?;

            head.shorthand().map(|s| s.into()).context("unable to get a branch name due to non-utf8 symbols")
        }
    }
}

pub fn normalize_field_names(fields: &[&str]) -> String {
    fields.iter()
        .map(|s| s.replace("r#", "")).collect::<Vec<String>>()
        .join(",")
}

pub fn get_build_type_by_path(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|s| s.to_str())
        .and_then(|name| CONFIG.teamcity.build_types.get(name).cloned())
}

