use std::collections::BTreeSet;
use std::path::Path;

use crate::model::{Capsule, CapsuleKind, Finding};
use crate::structure_validation::simple;

pub fn validate(root: &Path, capsule: &Capsule, findings: &mut Vec<Finding>) {
    if capsule.kind != CapsuleKind::Crate {
        return;
    }
    let path = root.join(&capsule.root).join("Cargo.toml");
    let Ok(bytes) = crate::repository_support::read_bounded(&path, 256 * 1024) else {
        return;
    };
    let Ok(manifest) = std::str::from_utf8(&bytes) else {
        return;
    };
    let declared: BTreeSet<_> = capsule
        .dependencies
        .iter()
        .map(|id| format!("lkjscript-{}", id.split('.').next().unwrap_or(id)))
        .collect();
    let mut in_dependencies = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if in_dependencies && name.starts_with("lkjscript-") && !declared.contains(name) {
            findings.push(simple(
                "error",
                "structure.capsule.actual-dependency",
                &capsule.root,
                &format!("actual dependency {name} is not declared by capsule"),
            ));
        }
    }
}
