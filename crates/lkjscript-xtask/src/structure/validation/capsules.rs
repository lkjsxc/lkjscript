use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::model::{Capsule, Finding};

use super::simple;

pub fn capsules(root: &Path, capsules: &[Capsule], findings: &mut Vec<Finding>) {
    let ids: BTreeSet<_> = capsules.iter().map(|capsule| capsule.id.as_str()).collect();
    let roots: BTreeSet<_> = capsules
        .iter()
        .map(|capsule| capsule.root.as_str())
        .collect();
    if ids.len() != capsules.len() || roots.len() != capsules.len() {
        findings.push(simple(
            "error",
            "LKJ-REPO-CAPSULE-ID",
            ".",
            "capsule IDs and roots must be unique",
        ));
    }
    let dependencies: BTreeMap<_, _> = capsules
        .iter()
        .map(|capsule| (capsule.id.as_str(), capsule.allowed_dependencies.as_slice()))
        .collect();
    for capsule in capsules {
        validate(root, capsule, &ids, &dependencies, findings);
        super::super::capsule_actual::validate(root, capsule, findings);
    }
}

fn validate<'a>(
    root: &Path,
    capsule: &'a Capsule,
    ids: &BTreeSet<&str>,
    dependencies: &BTreeMap<&'a str, &'a [String]>,
    findings: &mut Vec<Finding>,
) {
    if capsule.schema != "lkjscript.capsule" || capsule.version != 1 {
        findings.push(simple(
            "error",
            "LKJ-REPO-CAPSULE-SCHEMA",
            &capsule.root,
            "unsupported capsule schema or version",
        ));
    }
    let manifest = if capsule.root == "." {
        root.join("capsule.json")
    } else {
        root.join(&capsule.root).join("capsule.json")
    };
    if !manifest.is_file() {
        findings.push(simple(
            "error",
            "LKJ-REPO-CAPSULE-ID",
            &capsule.root,
            "capsule root does not own capsule.json",
        ));
    }
    if capsule.allowed_dependencies.len() > 16 || capsule.concepts.len() > 16 {
        findings.push(simple(
            "error",
            "LKJ-REPO-CAPSULE-FANOUT",
            &capsule.root,
            "capsule dependency or concept fanout exceeds 16",
        ));
    }
    let forbidden: BTreeSet<_> = capsule.forbidden_dependencies.iter().collect();
    for dependency in &capsule.allowed_dependencies {
        if !ids.contains(dependency.as_str()) || forbidden.contains(dependency) {
            findings.push(simple(
                "error",
                "LKJ-REPO-CAPSULE-DEPENDENCY",
                &capsule.root,
                &format!("invalid or forbidden allowed dependency {dependency}"),
            ));
        }
    }
    for dependency in &capsule.forbidden_dependencies {
        if !ids.contains(dependency.as_str()) {
            findings.push(simple(
                "error",
                "LKJ-REPO-CAPSULE-DEPENDENCY",
                &capsule.root,
                &format!("unknown forbidden dependency {dependency}"),
            ));
        }
    }
    if cycle(&capsule.id, &capsule.id, dependencies, &mut BTreeSet::new()) {
        findings.push(simple(
            "error",
            "LKJ-REPO-CAPSULE-CYCLE",
            &capsule.root,
            "capsule dependency cycle",
        ));
    }
    let evidence = capsule
        .facade
        .iter()
        .chain(&capsule.tests)
        .chain(&capsule.decisions);
    for path in evidence {
        if !root.join(path).is_file() {
            findings.push(simple(
                "error",
                "LKJ-REPO-CAPSULE-EVIDENCE",
                &capsule.root,
                &format!("capsule evidence path does not resolve exactly: {path}"),
            ));
        }
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
