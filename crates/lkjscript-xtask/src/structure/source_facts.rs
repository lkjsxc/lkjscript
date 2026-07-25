use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

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
    while index + 2 < lines.len() {
        if lines[index].trim() == "import/" && lines[index + 2].trim() == "/import" {
            let spec = lines[index + 1].trim().trim_matches('"');
            if let Some(target) = resolve_import(path, spec) {
                if tracked.contains(target.as_str()) {
                    edge(
                        edges,
                        unit,
                        &format!("source-unit:{target}"),
                        "imports",
                        &format!("{path}:{}-{}", index + 1, index + 3),
                        "inferred",
                    );
                }
            }
            index += 3;
        } else {
            index += 1;
        }
    }
}

fn resolve_import(source: &str, spec: &str) -> Option<String> {
    let combined = if let Some(relative) = spec.strip_prefix("./") {
        Path::new(source).parent()?.join(relative)
    } else if spec.starts_with("std/") || spec.starts_with("lib/") || spec.starts_with("examples/")
    {
        PathBuf::from("src").join(spec)
    } else {
        PathBuf::from(spec)
    };
    let mut result = PathBuf::new();
    for component in combined.components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    result.to_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_import_resolution_matches_source_loader_roots() {
        assert_eq!(
            super::resolve_import("src/examples/a.lkjscript", "std/io.lkjscript").as_deref(),
            Some("src/std/io.lkjscript")
        );
        assert_eq!(
            super::resolve_import("src/lib/a/main.lkjscript", "./part.lkjscript").as_deref(),
            Some("src/lib/a/part.lkjscript")
        );
    }
}
