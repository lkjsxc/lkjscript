mod repository;
mod scanner;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

const RULE: &str = "LKJ-UNSAFE-BOUNDARY";
const REGISTRY_PATH: &str = "meta/unsafe/registry.json";
const SCHEMA: &str = "lkjscript.unsafe-boundary-registry";
const MAX_BOUNDARIES: usize = 16;
const MAX_FILES_PER_BOUNDARY: usize = 16;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    boundaries: Vec<Boundary>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Boundary {
    id: String,
    responsibility: String,
    safe_caller_contract: String,
    files: Vec<String>,
}

pub fn run(root: &Path) -> i32 {
    let errors = validate(root);
    for error in &errors {
        eprintln!("{RULE} {error}");
    }
    if errors.is_empty() {
        println!("unsafe boundary registry is exact");
        0
    } else {
        1
    }
}
fn validate(root: &Path) -> Vec<String> {
    let registry = match load_registry(root) {
        Ok(registry) => registry,
        Err(error) => return vec![error],
    };
    let mut errors = validate_shape(&registry);
    let mut registered = BTreeMap::new();
    for boundary in &registry.boundaries {
        for file in &boundary.files {
            if registered
                .insert(file.clone(), boundary.id.clone())
                .is_some()
            {
                errors.push(format!("registered more than once: {file}"));
            }
            let path = root.join(file);
            if !repository::valid_path(file) {
                errors.push(format!("invalid repository-relative Rust path: {file}"));
            } else {
                match fs::read_to_string(&path) {
                    Ok(source) if scanner::contains_unsafe_token(&source) => {}
                    Ok(_) => {
                        errors.push(format!("registered file has no unsafe code token: {file}"))
                    }
                    Err(error) => {
                        errors.push(format!("cannot read registered file {file}: {error}"))
                    }
                }
            }
        }
    }
    match repository::rust_files(root) {
        Ok(files) => {
            let actual: BTreeSet<_> = files
                .into_iter()
                .filter_map(|(path, source)| {
                    scanner::contains_unsafe_token(&source).then_some(path)
                })
                .collect();
            for file in actual.difference(&registered.keys().cloned().collect()) {
                errors.push(format!("unsafe code token is not registered: {file}"));
            }
            for file in registered.keys() {
                if !actual.contains(file) {
                    errors.push(format!(
                        "registered path is not an unsafe token file: {file}"
                    ));
                }
            }
        }
        Err(error) => errors.push(error),
    }
    errors.sort();
    errors.dedup();
    errors
}
fn load_registry(root: &Path) -> Result<Registry, String> {
    let path = root.join(REGISTRY_PATH);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&source).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn validate_shape(registry: &Registry) -> Vec<String> {
    let mut errors = Vec::new();
    if registry.schema != SCHEMA {
        errors.push(format!("registry schema must be {SCHEMA}"));
    }
    if registry.boundaries.is_empty() || registry.boundaries.len() > MAX_BOUNDARIES {
        errors.push(format!(
            "registry must contain 1..={MAX_BOUNDARIES} boundaries"
        ));
    }
    let ids: Vec<_> = registry.boundaries.iter().map(|item| &item.id).collect();
    if !ids.windows(2).all(|pair| pair[0] < pair[1]) {
        errors.push("boundary IDs must be sorted and unique".into());
    }
    for boundary in &registry.boundaries {
        if !stable_id(&boundary.id) {
            errors.push(format!("invalid boundary ID: {}", boundary.id));
        }
        if boundary.responsibility.is_empty() || boundary.safe_caller_contract.is_empty() {
            errors.push(format!(
                "boundary {} has an empty contract field",
                boundary.id
            ));
        }
        if boundary.files.is_empty() || boundary.files.len() > MAX_FILES_PER_BOUNDARY {
            errors.push(format!(
                "boundary {} must contain 1..={MAX_FILES_PER_BOUNDARY} files",
                boundary.id
            ));
        }
        if !boundary.files.windows(2).all(|pair| pair[0] < pair[1]) {
            errors.push(format!(
                "boundary {} files must be sorted and unique",
                boundary.id
            ));
        }
    }
    errors
}
fn stable_id(value: &str) -> bool {
    value.len() <= 64
        && value.starts_with(|ch: char| ch.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
