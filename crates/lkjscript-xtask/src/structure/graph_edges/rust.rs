use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::model::{Audit, Edge, Node};

use super::{edge, node, read_text};
use crate::structure::graph::Budget;

pub fn add(
    root: &Path,
    audit: &Audit,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let tracked: BTreeSet<_> = audit.files.iter().map(|file| file.path.as_str()).collect();
    for file in audit.files.iter().filter(|file| file.path.ends_with(".rs")) {
        let Some(text) = read_text(root, &file.path, file.bytes, budget) else {
            break;
        };
        facts(&file.path, &text, &tracked, nodes, edges);
    }
}

fn facts(
    path: &str,
    text: &str,
    tracked: &BTreeSet<&str>,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let crate_id = crate_for(path);
    let mut test_attribute = None;
    let mut occurrences = BTreeMap::<String, u64>::new();
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        if line == "#[test]" {
            test_attribute = Some(line_number);
            continue;
        }
        if let Some(name) = function_name(line) {
            let evidence = format!("{path}:{line_number}");
            let prefix = if test_attribute.take().is_some() {
                "test"
            } else {
                "rust-symbol"
            };
            let occurrence = occurrences.entry(format!("{prefix}:{name}")).or_default();
            let id = format!("{prefix}:{path}:{name}:{occurrence}");
            *occurrence += 1;
            let kind = if prefix == "test" {
                "test"
            } else {
                "rust-symbol"
            };
            node(
                nodes,
                &id,
                kind,
                name,
                "authored",
                path,
                Some(evidence.clone()),
                "inferred",
            );
            edge(
                edges,
                &format!("file:{path}"),
                &id,
                "defines",
                &evidence,
                "inferred",
            );
            if kind == "test" {
                if let Some(crate_id) = &crate_id {
                    edge(edges, &id, crate_id, "tests", &evidence, "inferred");
                }
                edge(
                    edges,
                    "command:cargo-test-workspace",
                    &id,
                    "covers",
                    &evidence,
                    "inferred",
                );
            }
            continue;
        }
        if !line.starts_with("#[") {
            test_attribute = None;
        }
        rust_import(path, line, line_number, edges);
        if let Some(module) = module_name(line) {
            for target in module_targets(path, module) {
                if tracked.contains(target.as_str()) {
                    edge(
                        edges,
                        &format!("file:{path}"),
                        &format!("file:{target}"),
                        "imports",
                        &format!("{path}:{line_number}"),
                        "declared",
                    );
                    break;
                }
            }
        }
    }
}

fn rust_import(path: &str, line: &str, line_number: usize, edges: &mut Vec<Edge>) {
    let Some(import) = line.strip_prefix("use lkjscript_") else {
        return;
    };
    let name = import
        .split([':', ';', ' '])
        .next()
        .unwrap_or("")
        .replace('_', "-");
    edge(
        edges,
        &format!("file:{path}"),
        &format!("cargo-crate:lkjscript-{name}"),
        "imports",
        &format!("{path}:{line_number}"),
        "declared",
    );
}

fn function_name(line: &str) -> Option<&str> {
    let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
    let line = line.strip_prefix("pub ").unwrap_or(line);
    line.strip_prefix("fn ")?.split(['(', '<', ' ']).next()
}
fn module_name(line: &str) -> Option<&str> {
    let line = line.strip_prefix("pub ").unwrap_or(line);
    line.strip_prefix("mod ")?.strip_suffix(';')
}
fn crate_for(path: &str) -> Option<String> {
    let name = path.strip_prefix("crates/")?.split('/').next()?;
    Some(format!("cargo-crate:{name}"))
}
fn module_targets(path: &str, module: &str) -> [String; 2] {
    let parent = path.rsplit_once('/').map_or("", |(value, _)| value);
    [
        format!("{parent}/{module}.rs"),
        format!("{parent}/{module}/mod.rs"),
    ]
}
