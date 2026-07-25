use std::collections::BTreeSet;
use std::path::Path;

use crate::model::{Capsule, CapsuleKind, Finding};

use super::validation::simple;

pub fn validate(root: &Path, capsule: &Capsule, findings: &mut Vec<Finding>) {
    if capsule.kind != CapsuleKind::Crate {
        return;
    }
    let path = root.join(&capsule.root).join("Cargo.toml");
    let Ok(bytes) = super::repository_support::read_bounded(&path, 256 * 1024) else {
        return;
    };
    let Ok(manifest) = std::str::from_utf8(&bytes) else {
        return;
    };
    let declared: BTreeSet<_> = capsule.allowed_dependencies.iter().cloned().collect();
    for dependency in internal_dependencies(manifest) {
        let id = dependency.trim_start_matches("lkjscript-");
        if !declared.contains(id) {
            findings.push(simple(
                "error",
                "LKJ-REPO-CAPSULE-ACTUAL-DEPENDENCY",
                &capsule.root,
                &format!("actual Cargo dependency {dependency} is not allowed by capsule"),
            ));
        }
    }
}

pub(crate) fn internal_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    let mut dependency_section = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            dependency_section = line.ends_with("dependencies]")
                || line == "[dependencies]"
                || line == "[dev-dependencies]"
                || line == "[build-dependencies]";
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if dependency_section && name.starts_with("lkjscript-") {
            dependencies.insert(name.to_owned());
        }
    }
    dependencies
}

#[cfg(test)]
mod tests {
    #[test]
    fn extracts_normal_and_development_dependencies() {
        let manifest = "[dependencies]\nlkjscript-core = {}\n\
                        [dev-dependencies]\nlkjscript-native = {}\n";
        let dependencies = super::internal_dependencies(manifest);
        assert!(dependencies.contains("lkjscript-core"));
        assert!(dependencies.contains("lkjscript-native"));
    }
}
