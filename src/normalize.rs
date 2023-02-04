use anyhow::{Context, Result};
use skim::prelude::*;
use std::path::{Path, PathBuf};

pub fn normalize_path(path: Option<&Path>) -> std::io::Result<PathBuf> {
    match path {
        Some(p) => p.canonicalize(),
        None => std::env::current_dir(),
    }
}

pub fn normalize_branch_name(branch_name: Option<&str>, path: Option<&Path>) -> Result<String> {
    match branch_name {
        Some(bn) => Ok(bn.into()),
        None => {
            let p = normalize_path(path)?;
            let repo = git2::Repository::discover(p)?;
            let head = repo.head()?;

            head.shorthand()
                .map(|s| s.into())
                .context("unable to get a branch name due to non-utf8 symbols")
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

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    let result: Vec<T> = selected_items
        .iter()
        .filter_map(|v| (**v).as_any().downcast_ref())
        .map(|v: &T| v.clone())
        .collect::<Vec<T>>()
    ;

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
        .multi(true)
        .build()
        .unwrap();

    skim_select(data, &options)
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

    skim_select(data, &options)?.first().map(|v| v.clone()).context("No item was seleced")
}
