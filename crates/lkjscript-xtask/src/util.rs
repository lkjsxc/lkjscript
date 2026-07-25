use std::fs;
use std::path::{Path, PathBuf};

pub fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("read directory {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read entry in {}: {error}", dir.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if kind.is_dir() {
            walk(&path, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(value).map_err(|error| format!("serialize JSON: {error}"))?;
    println!("{text}");
    Ok(())
}
