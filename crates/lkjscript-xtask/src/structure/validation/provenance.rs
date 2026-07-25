use std::collections::BTreeSet;
use std::path::Path;

use crate::model::{FileRecord, Finding, Provenance};

use super::simple;

const RULE: &str = "LKJ-REPO-GENERATED-PROVENANCE";

pub fn provenance(
    root: &Path,
    files: &[FileRecord],
    entries: &[Provenance],
    findings: &mut Vec<Finding>,
) {
    let tracked: BTreeSet<_> = files.iter().map(|file| file.path.as_str()).collect();
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.path.contains(['*', '?', '[', ']']) {
            findings.push(simple(
                "error",
                RULE,
                &entry.path,
                "provenance path contains glob syntax",
            ));
        }
        if !matches!(
            entry.class.as_str(),
            "generated-mutable"
                | "immutable-license"
                | "immutable-benchmark-result"
                | "immutable-fixture"
        ) {
            findings.push(simple(
                "error",
                RULE,
                &entry.path,
                "unsupported provenance class",
            ));
        }
        if !seen.insert(entry.path.as_str()) {
            findings.push(simple("error", RULE, &entry.path, "duplicate exact path"));
            continue;
        }
        if !tracked.contains(entry.path.as_str()) {
            findings.push(simple("error", RULE, &entry.path, "path is not tracked"));
            continue;
        }
        match super::super::repository_support::read_bounded(
            &root.join(&entry.path),
            4 * 1024 * 1024,
        ) {
            Ok(bytes) if crate::sha256::digest(&bytes) != entry.sha256 => findings.push(simple(
                "error",
                RULE,
                &entry.path,
                "provenance hash does not match",
            )),
            Err(error) => findings.push(simple(
                "error",
                RULE,
                &entry.path,
                &format!("cannot read provenance input: {error}"),
            )),
            _ => {}
        }
        if entry.class == "generated-mutable" && entry.generator.as_deref().unwrap_or("").is_empty()
        {
            findings.push(simple(
                "error",
                RULE,
                &entry.path,
                "mutable generated entry lacks generator identity",
            ));
        }
    }
}
