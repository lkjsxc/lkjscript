use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::model::{Capsule, DirectoryRecord, FileRecord, Finding, Provenance};
use crate::repository_support::{
    capsule_for, classify, directories, finding, git, load_capsules, physical_lines, read_bounded,
};

const FILE_READ_CAP: u64 = 4 * 1024 * 1024;
const META_READ_CAP: u64 = 256 * 1024;
const GIT_OUTPUT_CAP: usize = 4 * 1024 * 1024;

pub struct Snapshot {
    pub revision: String,
    pub files: Vec<FileRecord>,
    pub directories: Vec<DirectoryRecord>,
    pub capsules: Vec<Capsule>,
    pub findings: Vec<Finding>,
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = read_bounded(path, META_READ_CAP)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

pub fn capture(root: &Path, provenance: &[Provenance]) -> Result<Snapshot, String> {
    let revision = git(root, &["rev-parse", "HEAD"])?;
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed".into());
    }
    if output.stdout.len() > GIT_OUTPUT_CAP {
        return Err("git ls-files output exceeds repository traversal bound".into());
    }
    let mut paths = Vec::new();
    for raw in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
    {
        let path = std::str::from_utf8(raw)
            .map_err(|_| "tracked path is not UTF-8")?
            .to_owned();
        paths.push(path);
    }
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate tracked path".into());
    }
    let classes: BTreeMap<_, _> = provenance
        .iter()
        .map(|entry| (entry.path.as_str(), entry.class.as_str()))
        .collect();
    let capsule_paths: BTreeSet<_> = paths
        .iter()
        .filter(|path| path.ends_with("lkjscript.capsule"))
        .cloned()
        .collect();
    let capsules = load_capsules(root, &capsule_paths)?;
    let mut findings = Vec::new();
    let mut files = Vec::new();
    for path in &paths {
        inspect_file(root, path, &classes, &capsules, &mut files, &mut findings)?;
    }
    if revision != git(root, &["rev-parse", "HEAD"])? {
        findings.push(finding(
            "error",
            "structure.repository.race",
            ".",
            "revision changed during audit",
        ));
    }
    Ok(Snapshot {
        revision,
        files,
        directories: directories(&paths)?,
        capsules,
        findings,
    })
}

fn inspect_file(
    root: &Path,
    path: &str,
    classes: &BTreeMap<&str, &str>,
    capsules: &[Capsule],
    files: &mut Vec<FileRecord>,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let absolute = root.join(path);
    let metadata =
        fs::symlink_metadata(&absolute).map_err(|error| format!("inspect {path}: {error}"))?;
    if metadata.file_type().is_symlink() {
        findings.push(finding(
            "error",
            "structure.path.symlink",
            path,
            "tracked symbolic link is forbidden",
        ));
        return Ok(());
    }
    if !metadata.is_file() {
        findings.push(finding(
            "error",
            "structure.path.unclassified",
            path,
            "tracked path is not a regular file",
        ));
        return Ok(());
    }
    if metadata.len() > FILE_READ_CAP {
        findings.push(finding(
            "error",
            "structure.file.read-bound",
            path,
            "tracked file exceeds audit read bound",
        ));
        return Ok(());
    }
    let bytes = read_bounded(&absolute, FILE_READ_CAP)?;
    let after = fs::metadata(&absolute).map_err(|error| format!("reinspect {path}: {error}"))?;
    if after.len() != metadata.len() {
        findings.push(finding(
            "error",
            "structure.repository.race",
            path,
            "file changed during audit",
        ));
    }
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => {
            findings.push(finding(
                "error",
                "structure.path.unclassified",
                path,
                "tracked file is not UTF-8",
            ));
            ""
        }
    };
    let width = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    files.push(FileRecord {
        path: path.into(),
        bytes: u64::try_from(bytes.len()).map_err(|_| "byte count overflow")?,
        lines: physical_lines(text)?,
        max_line_scalars: u64::try_from(width).map_err(|_| "width overflow")?,
        class: classes
            .get(path)
            .copied()
            .unwrap_or_else(|| classify(path))
            .into(),
        capsule: capsule_for(path, capsules),
    });
    Ok(())
}
