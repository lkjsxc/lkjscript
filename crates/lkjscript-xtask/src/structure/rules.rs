use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{Audit, ClassCount, Finding, Policy, Provenance, Ratchet};
use crate::repository::Snapshot;
use crate::structure_detector::content;
use crate::structure_validation::{capsules, metric, provenance, simple, warning};

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
                "structure.file.lines",
                &file.path,
                file.lines,
                policy.limits.authored_lines,
            );
            metric(
                &mut findings,
                "structure.file.bytes",
                &file.path,
                file.bytes,
                policy.limits.authored_bytes,
            );
            metric(
                &mut findings,
                "structure.line.scalars",
                &file.path,
                file.max_line_scalars,
                policy.limits.ordinary_line_scalars,
            );
            content(root, &file.path, &mut findings);
        }
    }
    for dir in &snapshot.directories {
        metric(
            &mut findings,
            "structure.directory.entries",
            &dir.path,
            dir.entries,
            policy.limits.directory_entries,
        );
        if dir.depth > policy.limits.error_depth {
            metric(
                &mut findings,
                "structure.directory.depth-error",
                &dir.path,
                dir.depth,
                policy.limits.error_depth,
            );
        } else if dir.depth > policy.limits.warning_depth {
            warning(
                &mut findings,
                "structure.directory.depth-warning",
                &dir.path,
                dir.depth,
                policy.limits.warning_depth,
                "directory depth warning",
            );
        }
        if dir.path != "." && dir.entries == 1 {
            warning(
                &mut findings,
                "structure.directory.one-child",
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
        schema: "lkjscript.structure.audit.v1".into(),
        revision: snapshot.revision,
        policy_version: policy.version.clone(),
        files: snapshot.files,
        directories: snapshot.directories,
        classifications,
        capsules: snapshot.capsules,
        findings,
        provenance: provenance_entries,
        unsupported: vec![
            "semantic top-level item analysis for non-Rust files".into(),
            "dynamic/runtime dependency discovery".into(),
            "macro-expanded Rust imports and tests".into(),
        ],
    }
}

pub fn check_findings(audit: &Audit, ratchet: &[Ratchet]) -> Vec<Finding> {
    let mut result: Vec<_> = audit
        .findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .cloned()
        .collect();
    let observed: BTreeMap<_, _> = result
        .iter()
        .filter_map(|finding| {
            finding
                .observed
                .map(|value| ((finding.rule.clone(), finding.path.clone()), value))
        })
        .collect();
    for record in ratchet {
        match observed.get(&(record.rule.clone(), record.path.clone())) {
            Some(value) if *value <= record.observed => {
                result.retain(|finding| finding.rule != record.rule || finding.path != record.path);
            }
            Some(_) => {}
            None => result.push(simple(
                "error",
                "structure.ratchet.stale",
                &record.path,
                &format!("stale ratchet record for {}", record.rule),
            )),
        }
    }
    result.sort_by(|left, right| left.sort_key.cmp(&right.sort_key));
    result
}

#[cfg(test)]
mod tests {
    use crate::model::{Audit, Finding, Ratchet};
    #[test]
    fn ratchet_allows_reduction_and_rejects_worsening() {
        let mut audit = empty();
        audit.findings.push(finding(201));
        let ratchet = vec![Ratchet {
            rule: "structure.file.lines".into(),
            path: "a".into(),
            observed: 202,
        }];
        assert!(super::check_findings(&audit, &ratchet).is_empty());
        audit.findings[0] = finding(203);
        assert_eq!(super::check_findings(&audit, &ratchet).len(), 1);
    }
    #[test]
    fn stale_ratchet_is_rejected() {
        let ratchet = vec![Ratchet {
            rule: "structure.file.lines".into(),
            path: "a".into(),
            observed: 201,
        }];
        assert_eq!(
            super::check_findings(&empty(), &ratchet)[0].rule,
            "structure.ratchet.stale"
        );
    }
    fn finding(value: u64) -> Finding {
        Finding {
            severity: "error".into(),
            rule: "structure.file.lines".into(),
            path: "a".into(),
            observed: Some(value),
            limit: Some(200),
            message: String::new(),
            provenance: None,
            sort_key: String::new(),
        }
    }
    fn empty() -> Audit {
        Audit {
            schema: String::new(),
            revision: String::new(),
            policy_version: String::new(),
            files: vec![],
            directories: vec![],
            classifications: vec![],
            capsules: vec![],
            findings: vec![],
            provenance: vec![],
            unsupported: vec![],
        }
    }
}
