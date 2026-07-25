use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{Audit, Edge, Node};

use super::{declared_node, edge, read_text};
use crate::structure::graph::Budget;

struct Manifest {
    path: String,
    package: String,
    dependencies: Vec<(String, usize)>,
}

pub fn add(
    root: &Path,
    audit: &Audit,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let mut manifests = Vec::new();
    for file in audit
        .files
        .iter()
        .filter(|file| file.path.ends_with("Cargo.toml"))
    {
        let Some(text) = read_text(root, &file.path, file.bytes, budget) else {
            break;
        };
        if let Some(manifest) = parse(&file.path, &text) {
            manifests.push(manifest);
        }
    }
    manifests.sort_by(|left, right| left.package.cmp(&right.package));
    let packages: BTreeMap<_, _> = manifests
        .iter()
        .map(|manifest| {
            (
                manifest.package.as_str(),
                format!("cargo-package:{}", manifest.package),
            )
        })
        .collect();
    for manifest in &manifests {
        let package_id = format!("cargo-package:{}", manifest.package);
        let crate_id = format!("cargo-crate:{}", manifest.package);
        declared_node(
            nodes,
            &package_id,
            "cargo-package",
            &manifest.package,
            &manifest.path,
        );
        declared_node(
            nodes,
            &crate_id,
            "cargo-crate",
            &manifest.package,
            &manifest.path,
        );
        edge(
            edges,
            &package_id,
            &crate_id,
            "contains",
            &manifest.path,
            "declared",
        );
        edge(
            edges,
            &crate_id,
            &format!("file:{}", manifest.path),
            "contains",
            &manifest.path,
            "declared",
        );
        for (dependency, line) in &manifest.dependencies {
            if let Some(target) = packages.get(dependency.as_str()) {
                edge(
                    edges,
                    &package_id,
                    target,
                    "depends-on",
                    &format!("{}:{line}", manifest.path),
                    "declared",
                );
            }
        }
    }
}

fn parse(path: &str, text: &str) -> Option<Manifest> {
    let mut package = None;
    let mut section = "";
    let mut dependencies = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = line;
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if section == "[package]" && name == "name" {
            package = quoted(value.trim());
        }
        let dependency_section = section.ends_with("dependencies]")
            || matches!(
                section,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
        if dependency_section && name.starts_with("lkjscript-") {
            dependencies.push((name.to_owned(), index + 1));
        }
    }
    Some(Manifest {
        path: path.into(),
        package: package?,
        dependencies,
    })
}

fn quoted(value: &str) -> Option<String> {
    value
        .strip_prefix('"')?
        .split('"')
        .next()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_manifest_parser_retains_dependency_line() {
        let manifest = super::parse(
            "crates/a/Cargo.toml",
            "[package]\nname = \"lkjscript-a\"\n[dependencies]\nlkjscript-core = {}\n",
        );
        assert_eq!(
            manifest.map(|value| value.dependencies),
            Some(vec![("lkjscript-core".into(), 4)])
        );
    }
}
