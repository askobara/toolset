use anyhow::{Context, Result};
use skim::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, Arc},
};

pub fn normalize_path(path: Option<&Path>) -> std::io::Result<PathBuf> {
    match path {
        Some(p) => p.canonicalize(),
        None => std::env::current_dir(),
    }
}

#[derive(Debug)]
pub struct BranchNameMeta {
    pub refname: String,
    pub local_name: String,
    pub upsteam_name: Option<String>,
    pub oid: git2::Oid,
    pub summary: Option<String>,
}

pub fn get_branch_name_meta(branch_name: Option<&str>, repo: &Repo) -> Result<BranchNameMeta> {
    match branch_name {
        Some(_) => todo!(),
        None => {
            let repo = repo.lock().unwrap();
            let head = repo.head()?;

            if !head.is_branch() {
                anyhow::bail!("HEAD is not a branch");
            }

            let oid = head.target().context("HEAD is symbolic one")?;
            let summary = repo.find_commit(oid).ok().and_then(|c| c.summary().map(ToOwned::to_owned));

            let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

            let r: Option<String> = repo.branch_upstream_name(refname)
                .ok()
                .and_then(|b| b.as_str().map(ToOwned::to_owned))
                .and_then(normalize)
            ;

            Ok(BranchNameMeta {
                refname: refname.to_owned(),
                local_name: normalize(refname).context("Not utf-8")?,
                upsteam_name: r,
                oid,
                summary,
            })
        }
    }
}

fn normalize(s: impl Into<String>) -> Option<String> {
    Path::new(&s.into())
        .file_stem()
        .and_then(|s| s.to_str().map(ToOwned::to_owned))
}

pub fn normalize_branch_name(branch_name: Option<&str>, repo: &Repo) -> Result<String> {
    match branch_name {
        Some(bn) => Ok(bn.into()),
        None => {
            let repo = repo.lock().unwrap();
            let head = repo.head()?;

            let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

            let r = repo.branch_upstream_name(refname)
                .ok()
                .and_then(|b| b.as_str().map(String::from))
                .unwrap_or(refname.to_owned())
            ;

            Path::new(&r)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(ToOwned::to_owned)
                .context("Cannot get branch name")
        }
    }
}

pub fn normalize_field_names(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|s| s.replace("r#", ""))
        .collect::<Vec<String>>()
        .join(",")
}

fn skim_select<I, T>(data: I, options: &SkimOptions) -> Result<Vec<T>>
where
    T: SkimItem + Clone,
    I: IntoIterator<Item = T>,
{
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    for item in data {
        let _ = tx_item.send(Arc::new(item));
    }

    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_items = Skim::run_with(options, Some(rx_item))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    let result: Vec<T> = selected_items
        .iter()
        .filter_map(|v| (**v).as_any().downcast_ref())
        .cloned()
        .collect::<Vec<T>>();

    Ok(result)
}

pub fn select_many<I, T>(data: I, query: Option<&str>) -> Result<Vec<T>>
where
    T: SkimItem + Clone,
    I: IntoIterator<Item = T>,
{
    let options = SkimOptionsBuilder::default()
        .height(Some("20%"))
        .query(query)
        .select1(query.is_some())
        .multi(true)
        .bind(vec!["ctrl-a:beginning-of-line", "ctrl-e:end-of-line"])
        .build()
        .unwrap();

    skim_select(data, &options).and_then(|arr| {
        if !arr.is_empty() {
            Ok(arr)
        } else {
            anyhow::bail!("No items selected")
        }
    })
}

pub fn select_one<I, T>(data: I, query: Option<&str>) -> Result<T>
where
    T: SkimItem + Clone,
    I: IntoIterator<Item = T>,
{
    let options = SkimOptionsBuilder::default()
        .height(Some("20%"))
        .query(query)
        .select1(query.is_some())
        .build()
        .unwrap();

    skim_select(data, &options)?
        .first()
        .cloned()
        .context("No item was selected")
}

pub type Repo = Arc<Mutex<git2::Repository>>;

pub fn find_a_repo(path: Option<&Path>) -> Result<Repo> {
    let path = normalize_path(path)?;
    let repo = git2::Repository::discover(path)?;

    Ok(Arc::new(Mutex::new(repo)))
}

pub fn get_repo_name(repo: &Repo, remote_name: Option<&str>) -> Result<String> {
    let repo = repo.lock().unwrap();
    let remote = repo.find_remote(remote_name.unwrap_or("origin"))?;
    let url = remote.url().context("Remote url contains non-utf8 symbols")?;

    Path::new(url)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_owned())
        .context("Cannot get repo name")
}
