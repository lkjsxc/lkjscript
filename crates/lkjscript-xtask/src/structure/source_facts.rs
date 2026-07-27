use std::collections::BTreeSet;
use std::path::Path;

use lkjscript_compiler::source;
use lkjscript_core::Limits;

use crate::model::{Audit, Edge, Node};

use super::graph::Budget;
use super::graph_edges::{edge, node, read_text};

pub fn add(
    root: &Path,
    audit: &Audit,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let tracked: BTreeSet<_> = audit
        .files
        .iter()
        .filter(|file| file.path.ends_with(".lkjscript"))
        .map(|file| file.path.as_str())
        .collect();
    for file in audit
        .files
        .iter()
        .filter(|file| file.path.ends_with(".lkjscript"))
    {
        let Some(text) = read_text(root, &file.path, file.bytes, budget) else {
            break;
        };
        let unit = format!("source-unit:{}", file.path);
        node(
            nodes,
            &unit,
            "source-unit",
            &file.path,
            "authored",
            &file.path,
            Some(format!("{}:bytes=0..{}", file.path, file.bytes)),
            "compiler-derived",
        );
        edge(
            edges,
            &format!("file:{}", file.path),
            &unit,
            "defines",
            &file.path,
            "declared",
        );
        edge(
            edges,
            &unit,
            "command:check-sources",
            "validated-by",
            "source_checks::check_sources",
            "declared",
        );
        declarations(&file.path, &text, &unit, nodes, edges);
        imports(&file.path, &text, &unit, &tracked, edges);
    }
}

fn declarations(path: &str, text: &str, unit: &str, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>) {
    let Ok(tree) = source::validate(text, path, &Limits::default()) else {
        return;
    };
    for declaration in tree.declarations() {
        let span = declaration.span();
        let evidence = format!(
            "{}:{}:{}-{}:{};bytes={}..{}",
            declaration.origin().logical_path(),
            span.start().line(),
            span.start().column(),
            span.end().line(),
            span.end().column(),
            span.start().byte(),
            span.end().byte()
        );
        let id = format!("lkjscript-declaration:{}", declaration.key().to_hex());
        node(
            nodes,
            &id,
            "lkjscript-declaration",
            &format!("{} {}", declaration.kind().as_str(), declaration.name()),
            "authored",
            declaration.origin().logical_path(),
            Some(evidence.clone()),
            "compiler-derived",
        );
        edge(edges, unit, &id, "declares", &evidence, "compiler-derived");
        if declaration.kind() == source::DeclarationKind::Implementation {
            edge(
                edges,
                unit,
                &id,
                "implements",
                &evidence,
                "compiler-derived",
            );
        }
    }
}

fn imports(path: &str, text: &str, unit: &str, tracked: &BTreeSet<&str>, edges: &mut Vec<Edge>) {
    let lines: Vec<_> = text.lines().collect();
    let mut index = 0;
    while index + 6 < lines.len() {
        if lines[index].trim() != "import/" || lines[index + 1].trim() != "module/" {
            index += 1;
            continue;
        }
        let spec = lines[index + 2].trim();
        if lines[index + 3].trim() != "/module" || lines[index + 4].trim() != "declarations/" {
            index += 1;
            continue;
        }
        let Some(close) = lines[index + 5..]
            .iter()
            .position(|line| line.trim() == "/declarations")
            .map(|offset| index + 5 + offset)
        else {
            index += 1;
            continue;
        };
        if lines.get(close + 1).map(|line| line.trim()) != Some("/import") {
            index += 1;
            continue;
        }
        if let Some(target) = resolve_import(spec) {
            if tracked.contains(target.as_str()) {
                edge(
                    edges,
                    unit,
                    &format!("source-unit:{target}"),
                    "imports",
                    &format!("{path}:{}-{}", index + 1, close + 2),
                    "inferred",
                );
            }
        }
        index = close + 2;
    }
}

fn resolve_import(spec: &str) -> Option<String> {
    let path = Path::new(spec);
    if path.is_absolute()
        || spec.starts_with('.')
        || !spec.ends_with(".lkjscript")
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }
    Some(spec.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_import_resolution_matches_package_module_ids() {
        assert_eq!(
            super::resolve_import("src/std/io.lkjscript").as_deref(),
            Some("src/std/io.lkjscript")
        );
        assert!(super::resolve_import("./part.lkjscript").is_none());
        assert!(super::resolve_import("../part.lkjscript").is_none());
    }
}
