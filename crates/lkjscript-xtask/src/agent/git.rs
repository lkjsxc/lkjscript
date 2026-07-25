use std::path::Path;
use std::process::{Command, Output};

use super::bounds;

pub fn head(root: &Path) -> Result<String, String> {
    text(root, &["rev-parse", "HEAD"])
}

pub fn require_commit(root: &Path, revision: &str) -> Result<(), String> {
    revision_text(revision)?;
    let object = format!("{revision}^{{commit}}");
    let output = run(root, &["cat-file", "-e", &object])?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("repository revision is not a commit: {revision}"))
    }
}

pub fn require_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<(), String> {
    require_commit(root, ancestor)?;
    require_commit(root, descendant)?;
    let output = run(root, &["merge-base", "--is-ancestor", ancestor, descendant])?;
    if output.status.success() {
        Ok(())
    } else if output.status.code() == Some(1) {
        Err(format!(
            "repository revision {ancestor} is not an ancestor of {descendant}"
        ))
    } else {
        Err("git merge-base failed".into())
    }
}

pub fn capsule_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = run(root, &["ls-files", "-z", "*capsule.json"])?;
    if !output.status.success() {
        return Err("git ls-files for capsules failed".into());
    }
    let mut paths = Vec::new();
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
    {
        let path = std::str::from_utf8(bytes).map_err(|_| "capsule path is not UTF-8")?;
        paths.push(path.to_owned());
        if paths.len() > bounds::CAPSULE_FILES {
            return Err("capsule enumeration exceeds bound".into());
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub fn tracked(root: &Path, path: &str) -> Result<bool, String> {
    let output = run(root, &["ls-files", "--error-unmatch", "--", path])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("git ls-files failed for {path}")),
    }
}

pub fn revision_text(value: &str) -> Result<(), String> {
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("invalid repository revision: {value}"));
    }
    Ok(())
}

fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = run(root, args)?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")));
    }
    let value = std::str::from_utf8(&output.stdout).map_err(|_| "git output is not UTF-8")?;
    Ok(value.trim().to_owned())
}

fn run(root: &Path, args: &[&str]) -> Result<Output, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))?;
    let bytes = output
        .stdout
        .len()
        .checked_add(output.stderr.len())
        .ok_or("git output count overflow")?;
    if bytes > bounds::GIT_OUTPUT_BYTES {
        return Err("git output exceeds bound".into());
    }
    Ok(output)
}
