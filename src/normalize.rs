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

pub fn normalize(s: impl Into<String>) -> Option<String> {
    Path::new(&s.into())
        .file_stem()
        .and_then(|s| s.to_str().map(ToOwned::to_owned))
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
