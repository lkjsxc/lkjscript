use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::read_revision;

pub(super) fn comparison(root: &Path) -> Result<(Option<u64>, Vec<PathBuf>), String> {
    let status = run(root, &["status", "--porcelain", "--untracked-files=all"])?;
    if !status.trim().is_empty() {
        let changed = status
            .lines()
            .filter_map(|line| line.get(3..))
            .map(PathBuf::from)
            .collect();
        return Ok((revision(root, "HEAD")?, changed));
    }
    let parents = run(root, &["rev-list", "--parents", "-n", "1", "HEAD"])?;
    let fields: Vec<_> = parents.split_whitespace().collect();
    if fields.len() == 1 {
        return Ok((None, vec![PathBuf::from("meta/platform-revision")]));
    }
    let parent = fields
        .get(1)
        .ok_or_else(|| "missing first Git parent".to_string())?;
    let changed = run(
        root,
        &["diff", "--name-only", "--no-renames", parent, "HEAD"],
    )?
    .lines()
    .map(PathBuf::from)
    .collect();
    Ok((revision(root, parent)?, changed))
}

fn revision(root: &Path, revision: &str) -> Result<Option<u64>, String> {
    let spec = format!("{revision}:meta/platform-revision");
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &spec])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Ok(None);
    }
    let temporary = root.join("target/lkjscript/platform-parent-revision");
    if let Some(parent) = temporary.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&temporary, &output.stdout).map_err(|error| error.to_string())?;
    let result = read_revision(&temporary);
    let _ = fs::remove_file(temporary);
    result.map(Some)
}

fn run(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || output.stdout.len() > 65_536 {
        return Err(format!(
            "bounded git command failed: git {}",
            arguments.join(" ")
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}
