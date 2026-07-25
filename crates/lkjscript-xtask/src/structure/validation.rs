use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::model::{Capsule, FileRecord, Finding, Provenance};

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
                "structure.provenance.exact-path",
                &entry.path,
                "provenance paths must be exact and cannot contain glob syntax",
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
                "structure.provenance.class",
                &entry.path,
                "unsupported provenance class",
            ));
        }
        if !seen.insert(entry.path.as_str()) {
            findings.push(simple(
                "error",
                "structure.provenance.duplicate",
                &entry.path,
                "duplicate exact provenance path",
            ));
            continue;
        }
        if !tracked.contains(entry.path.as_str()) {
            findings.push(simple(
                "error",
                "structure.provenance.missing",
                &entry.path,
                "provenance path is not tracked",
            ));
            continue;
        }
        match crate::repository_support::read_bounded(&root.join(&entry.path), 4 * 1024 * 1024) {
            Ok(bytes) if crate::sha256::digest(&bytes) != entry.sha256 => findings.push(simple(
                "error",
                "structure.provenance.stale",
                &entry.path,
                "provenance hash does not match",
            )),
            Err(error) => findings.push(simple(
                "error",
                "structure.provenance.read",
                &entry.path,
                &format!("cannot read provenance input: {error}"),
            )),
            _ => {}
        }
        if entry.class == "generated-mutable" && entry.generator.as_deref().unwrap_or("").is_empty()
        {
            findings.push(simple(
                "error",
                "structure.provenance.generator",
                &entry.path,
                "mutable generated entry lacks generator identity",
            ));
        }
    }
}

pub fn capsules(root: &Path, capsules: &[Capsule], findings: &mut Vec<Finding>) {
    let ids: BTreeSet<_> = capsules.iter().map(|capsule| capsule.id.as_str()).collect();
    if ids.len() != capsules.len() {
        findings.push(simple(
            "error",
            "structure.capsule.duplicate",
            ".",
            "duplicate capsule id",
        ));
    }
    let dependencies: BTreeMap<_, _> = capsules
        .iter()
        .map(|capsule| (capsule.id.as_str(), capsule.dependencies.as_slice()))
        .collect();
    for capsule in capsules {
        if capsule.schema != "lkjscript.capsule" || capsule.version != 1 {
            findings.push(simple(
                "error",
                "structure.capsule.version",
                &capsule.root,
                "unsupported capsule schema or version",
            ));
        }
        if capsule.dependencies.len() > 16 {
            findings.push(simple(
                "error",
                "structure.capsule.fanout",
                &capsule.root,
                "capsule dependency fanout exceeds 16",
            ));
        }
        for dependency in &capsule.dependencies {
            if !ids.contains(dependency.as_str()) {
                findings.push(simple(
                    "error",
                    "structure.capsule.dependency",
                    &capsule.root,
                    &format!("unknown capsule dependency {dependency}"),
                ));
            }
        }
        if cycle(
            &capsule.id,
            &capsule.id,
            &dependencies,
            &mut BTreeSet::new(),
        ) {
            findings.push(simple(
                "error",
                "structure.capsule.cycle",
                &capsule.root,
                "capsule dependency cycle",
            ));
        }
        crate::capsule_actual::validate(root, capsule, findings);
    }
}

pub(crate) fn cycle<'a>(
    start: &str,
    current: &'a str,
    graph: &BTreeMap<&'a str, &'a [String]>,
    seen: &mut BTreeSet<&'a str>,
) -> bool {
    if !seen.insert(current) {
        return false;
    }
    graph.get(current).is_some_and(|next| {
        next.iter()
            .any(|item| item == start || cycle(start, item, graph, seen))
    })
}

pub fn metric(findings: &mut Vec<Finding>, rule: &str, path: &str, observed: u64, limit: u64) {
    if observed > limit {
        findings.push(Finding {
            severity: "error".into(),
            rule: rule.into(),
            path: path.into(),
            observed: Some(observed),
            limit: Some(limit),
            message: format!("observed {observed}, limit {limit}"),
            provenance: None,
            sort_key: format!("error:{rule}:{path}:{observed:020}"),
        });
    }
}
pub fn warning(
    findings: &mut Vec<Finding>,
    rule: &str,
    path: &str,
    observed: u64,
    limit: u64,
    message: &str,
) {
    findings.push(Finding {
        severity: "warning".into(),
        rule: rule.into(),
        path: path.into(),
        observed: Some(observed),
        limit: Some(limit),
        message: message.into(),
        provenance: None,
        sort_key: format!("warning:{rule}:{path}:{observed:020}"),
    });
}
pub fn simple(severity: &str, rule: &str, path: &str, message: &str) -> Finding {
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
