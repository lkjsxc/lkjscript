use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use crate::model::{Capsule, DirectoryRecord, Finding};

pub fn load_capsules(root: &Path, paths: &BTreeSet<String>) -> Result<Vec<Capsule>, String> {
    let mut result: Vec<Capsule> = Vec::new();
    for path in paths {
        result.push(crate::repository::load_json(&root.join(path))?);
    }
    result.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(result)
}

pub fn directories(paths: &[String]) -> Result<Vec<DirectoryRecord>, String> {
    let mut children: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in paths {
        let parts: Vec<_> = path.split('/').collect();
        for index in 0..parts.len() {
            let parent = if index == 0 {
                ".".into()
            } else {
                parts[..index].join("/")
            };
            children
                .entry(parent)
                .or_default()
                .insert(parts[index].into());
        }
    }
    children
        .into_iter()
        .map(|(path, entries)| {
            Ok(DirectoryRecord {
                depth: if path == "." {
                    0
                } else {
                    u64::try_from(path.split('/').count()).map_err(|_| "depth overflow")?
                },
                path,
                entries: u64::try_from(entries.len()).map_err(|_| "entry count overflow")?,
            })
        })
        .collect()
}

pub fn physical_lines(text: &str) -> Result<u64, String> {
    if text.is_empty() {
        return Ok(0);
    }
    let count = text
        .as_bytes()
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();
    let total = count
        .checked_add(usize::from(!text.ends_with('\n')))
        .ok_or("line count overflow")?;
    u64::try_from(total).map_err(|_| "line count overflow".into())
}

pub fn classify(path: &str) -> &'static str {
    if path.ends_with("lkjscript.capsule") {
        "capsule-manifest"
    } else if path == "meta/structure/ratchet.json" {
        "generated-migration-state"
    } else {
        "authored"
    }
}

pub fn capsule_for(path: &str, capsules: &[Capsule]) -> Option<String> {
    capsules
        .iter()
        .filter(|capsule| {
            capsule.root == "."
                || path == capsule.root
                || path.starts_with(&format!("{}/", capsule.root))
        })
        .max_by_key(|capsule| capsule.root.len())
        .map(|capsule| capsule.id.clone())
}

pub fn read_bounded(path: &Path, cap: u64) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let take = cap.checked_add(1).ok_or("read cap overflow")?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(take)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    if u64::try_from(bytes.len()).map_err(|_| "read size overflow")? > cap {
        return Err(format!("{} exceeds read cap", path.display()));
    }
    Ok(bytes)
}

pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("run git: {error}"))?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().into())
        .map_err(|_| "git output is not UTF-8".into())
}

pub fn finding(severity: &str, rule: &str, path: &str, message: &str) -> Finding {
    Finding {
        severity: severity.into(),
        rule: rule.into(),
        path: path.into(),
        observed: None,
        limit: None,
        message: message.into(),
        provenance: None,
        sort_key: format!("{severity}:{rule}:{path}"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn physical_line_boundaries() {
        assert_eq!(super::physical_lines(&"x\n".repeat(199)).ok(), Some(199));
        assert_eq!(super::physical_lines(&"x\n".repeat(200)).ok(), Some(200));
        assert_eq!(super::physical_lines(&"x\n".repeat(201)).ok(), Some(201));
    }
    #[test]
    fn checked_overflow_is_rejected() {
        assert!(u64::MAX.checked_add(1).is_none());
    }
    #[test]
    fn directory_entry_boundaries() {
        for count in [15_usize, 16, 17] {
            let paths: Vec<_> = (0..count).map(|index| format!("d/{index:02}")).collect();
            let dirs = super::directories(&paths);
            assert!(dirs.is_ok());
            let observed = dirs
                .ok()
                .and_then(|items| items.into_iter().find(|item| item.path == "d"));
            assert_eq!(observed.map(|item| item.entries), Some(count as u64));
        }
    }
}
