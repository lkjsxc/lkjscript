use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::scan_marker::{scan_file, Claims};

const MAX_FILES: usize = 2_048;
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CLAIMS: usize = 4_096;

pub fn claims(root: &Path) -> (Claims, usize) {
    let mut paths = Vec::new();
    let mut observed = 0usize;
    let mut failures = 0usize;
    match fs::read_dir(root) {
        Ok(entries) => {
            for entry in entries.flatten() {
                observed += 1;
                if observed > MAX_FILES {
                    failures += 1;
                    break;
                }
                let markdown = entry.file_type().is_ok_and(|kind| kind.is_file())
                    && entry.path().extension().and_then(|value| value.to_str()) == Some("md");
                if markdown {
                    paths.push(entry.path());
                }
            }
        }
        Err(_) => failures += 1,
    }
    let docs = root.join("docs");
    if failures == 0 {
        let contained_docs = root.canonicalize().ok().zip(docs.canonicalize().ok());
        let traversal = match contained_docs {
            Some((base, child)) if child.starts_with(&base) => {
                walk_bounded(&docs, &mut paths, &mut observed, 0)
            }
            _ => Err("documentation root escapes repository".into()),
        };
        failures += usize::from(traversal.is_err());
    }
    paths.sort();
    if failures > 0 {
        eprintln!("documentation input traversal limit or read failed");
        return (BTreeMap::new(), failures);
    }
    let mut charged = 0u64;
    let mut result = BTreeMap::new();
    for path in paths {
        let remaining = MAX_BYTES.saturating_sub(charged);
        let Ok(bytes) = crate::structure::repository_support::read_bounded(&path, remaining) else {
            eprintln!("documentation input byte limit exceeded");
            failures += 1;
            break;
        };
        let Ok(length) = u64::try_from(bytes.len()) else {
            failures += 1;
            break;
        };
        charged += length;
        let Ok(content) = std::str::from_utf8(&bytes) else {
            failures += 1;
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
        failures += scan_file(&relative, content, &mut result);
        if result.len() > MAX_CLAIMS {
            eprintln!("documentation claim limit exceeded");
            failures += 1;
            break;
        }
    }
    (result, failures)
}

fn walk_bounded(
    directory: &Path,
    paths: &mut Vec<std::path::PathBuf>,
    observed: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > 16 {
        return Err("documentation traversal depth exceeded".into());
    }
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        *observed = observed
            .checked_add(1)
            .ok_or("documentation entry count overflow")?;
        if *observed > MAX_FILES {
            return Err("documentation entry count exceeded".into());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            walk_bounded(&entry.path(), paths, observed, depth + 1)?;
        } else if kind.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("md")
        {
            paths.push(entry.path());
        }
    }
    Ok(())
}
