use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) fn valid_path(value: &str) -> bool {
    value.ends_with(".rs")
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(super) fn rust_files(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut paths = Vec::new();
    collect_rust(&root.join("crates"), &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(Path::to_str)
                .ok_or_else(|| format!("non-UTF-8 Rust path: {}", path.display()))?
                .replace('\\', "/");
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read Rust file {relative}: {error}"))?;
            Ok((relative, source))
        })
        .collect()
}

fn collect_rust(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read directory {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read directory entry: {error}"))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if kind.is_dir() {
            collect_rust(&path, files)?;
        } else if kind.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs")
        {
            files.push(path);
        }
    }
    Ok(())
}
