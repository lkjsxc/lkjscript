use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{Audit, ClassCount, Finding, Policy, Provenance};

use super::repository::Snapshot;
use super::validation::{capsules, metric, provenance, simple, warning};

pub fn audit(
    root: &Path,
    policy: &Policy,
    mut provenance_entries: Vec<Provenance>,
    snapshot: Snapshot,
) -> Audit {
    provenance_entries.sort_by(|left, right| left.path.cmp(&right.path));
    let mut findings = snapshot.findings;
    for file in &snapshot.files {
        if file.class == "authored" {
            metric(
                &mut findings,
                "LKJ-REPO-FILE-LINES",
                &file.path,
                file.lines,
                policy.limits.authored_lines,
            );
            metric(
                &mut findings,
                "LKJ-REPO-FILE-BYTES",
                &file.path,
                file.bytes,
                policy.limits.authored_bytes,
            );
            metric(
                &mut findings,
                "LKJ-REPO-LINE-WIDTH",
                &file.path,
                file.max_ordinary_line_scalars,
                policy.limits.ordinary_line_scalars,
            );
            super::detector::content(root, &file.path, &mut findings);
            vague_module(&file.path, &mut findings);
        }
    }
    for dir in &snapshot.directories {
        metric(
            &mut findings,
            "LKJ-REPO-DIR-WIDTH",
            &dir.path,
            dir.entries,
            policy.limits.directory_entries,
        );
        if dir.depth > policy.limits.error_depth {
            metric(
                &mut findings,
                "LKJ-REPO-DIR-DEPTH",
                &dir.path,
                dir.depth,
                policy.limits.error_depth,
            );
        } else if dir.depth > policy.limits.warning_depth {
            warning(
                &mut findings,
                "LKJ-REPO-DIR-DEPTH",
                &dir.path,
                dir.depth,
                policy.limits.warning_depth,
                "directory depth warning",
            );
        }
        if dir.path != "." && dir.entries == 1 {
            warning(
                &mut findings,
                "LKJ-REPO-DIR-ONE-CHILD",
                &dir.path,
                1,
                1,
                "directory has one tracked immediate child",
            );
        }
    }
    provenance(root, &snapshot.files, &provenance_entries, &mut findings);
    capsules(root, &snapshot.capsules, &mut findings);
    findings.sort_by(|left, right| left.sort_key.cmp(&right.sort_key));
    let mut counts = BTreeMap::<String, u64>::new();
    for file in &snapshot.files {
        *counts.entry(file.class.clone()).or_default() += 1;
    }
    let classifications = counts
        .into_iter()
        .map(|(class, files)| ClassCount { class, files })
        .collect();
    Audit {
        schema: "lkjscript.repository-audit".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        revision: snapshot.revision,
        policy_identity: policy.contract.clone(),
        files: snapshot.files,
        directories: snapshot.directories,
        classifications,
        capsules: snapshot.capsules,
        findings,
        provenance: provenance_entries,
        unsupported: vec![
            "macro-expanded Rust item and dependency analysis".into(),
            "dynamic dispatch and runtime-loaded dependency discovery".into(),
            "non-Rust top-level item counting except validated lkjscript declarations".into(),
        ],
    }
}

fn vague_module(path: &str, findings: &mut Vec<Finding>) {
    let Some(name) = path.rsplit('/').next() else {
        return;
    };
    if matches!(
        name,
        "legacy_docs.rs"
            | "legacy_run.rs"
            | "legacy_sources.rs"
            | "docs.rs"
            | "run.rs"
            | "sources.rs"
    ) {
        findings.push(simple(
            "error",
            "LKJ-REPO-VAGUE-MODULE",
            path,
            "module name does not identify its precise responsibility",
        ));
    }
}

pub fn check_findings(audit: &Audit) -> Vec<Finding> {
    audit
        .findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .cloned()
        .collect()
}
