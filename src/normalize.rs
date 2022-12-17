use std::path::{Path, PathBuf};
use crate::CONFIG;

pub fn normalize_path(path: &Option<String>) -> PathBuf {
    let path_buf = match path {
        Some(p) => Path::new(p).to_owned(),
        None => std::env::current_dir().unwrap()
    };

    path_buf.canonicalize().unwrap()
}

pub fn normalize_branch_name(branch_name: &Option<String>, path: &Path) -> String {
    branch_name.as_deref().map(|s| s.to_string()).unwrap_or_else(|| {
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap();
        head.shorthand().map(|s| s.to_string()).unwrap()
    })
}

pub fn normalize_build_type(build_type: &Option<String>, path: &Path) -> String {
    build_type.as_deref().map(|s| s.to_string()).unwrap_or_else(|| {
        get_build_type_by_path(path)
    })
}

pub fn normalize_field_names(fields: &[&str]) -> String {
    fields.into_iter()
        .map(|s| s.replace("r#", "")).collect::<Vec<String>>()
        .join(",")
}

pub fn get_build_type_by_path(path: &Path) -> String {
    let basename = path.file_name().unwrap().to_str().unwrap();
    CONFIG.build_types.get(basename).unwrap().to_string()
}

