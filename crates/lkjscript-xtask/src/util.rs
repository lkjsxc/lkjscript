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
    let text = json_pretty(value)?;
    println!("{text}");
    Ok(())
}

pub fn print_json_bounded<T: serde::Serialize>(value: &T, limit: u64) -> Result<(), String> {
    let text = json_pretty(value)?;
    let bytes = u64::try_from(text.len()).map_err(|_| "serialized JSON size overflow")?;
    if bytes > limit {
        return Err(format!(
            "serialized JSON exceeds byte limit: {bytes} > {limit}"
        ));
    }
    println!("{text}");
    Ok(())
}

pub fn json_pretty_len<T: serde::Serialize>(value: &T) -> Result<u64, String> {
    let text = json_pretty(value)?;
    u64::try_from(text.len()).map_err(|_| "serialized JSON size overflow".into())
}

fn json_pretty<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| format!("serialize JSON: {error}"))
}
