use crate::hir::{Operation, Type};
use crate::source::{SourceNode, SourceResult, SourceSpan, SyntaxKind, ValidatedSourceTree};

use super::resolution_hir::{collect_resolved, ResolvedOperation};
use crate::source::migration::diagnostic;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::source::migration) struct ConversionInsertion {
    pub file: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
struct SourceOperation {
    file: usize,
    name: String,
    arguments: Vec<SourceSpan>,
}

pub(in crate::source::migration) fn resolved_conversions(
    tree: &ValidatedSourceTree,
) -> SourceResult<Vec<ConversionInsertion>> {
    let program = crate::analyze::analyze_program(tree)
        .map_err(|error| diagnostic(tree, "LKJ-SRC-MIGRATION-SEMANTICS", error.to_string()))?;
    let mut resolved = Vec::new();
    for function in &program.functions {
        collect_resolved(&function.body, &mut resolved);
    }
    collect_resolved(&program.main.body, &mut resolved);
    let source = source_operations(tree);
    if source.len() != resolved.len() {
        return Err(diagnostic(
            tree,
            "LKJ-SRC-MIGRATION-RESOLUTION",
            format!(
                "resolved numeric operation count {} does not match source count {}",
                resolved.len(),
                source.len()
            ),
        ));
    }
    let mut insertions = Vec::new();
    for (source, resolved) in source.iter().zip(&resolved) {
        resolve_operation(tree, source, resolved, &mut insertions)?;
    }
    insertions.sort_by_key(|site| (site.file, site.start, site.end));
    insertions.dedup();
    Ok(insertions)
}

fn source_operations(tree: &ValidatedSourceTree) -> Vec<SourceOperation> {
    let mut source = Vec::new();
    for (file, unit) in tree.files().iter().enumerate() {
        for form in &unit.syntax {
            if matches!(&form.kind, SyntaxKind::Call { name } if name == "def") {
                collect_source(form, file, &mut source);
            }
        }
    }
    let root = tree.root_path();
    for (file, unit) in tree.files().iter().enumerate() {
        if unit.path == root {
            for form in &unit.syntax {
                if matches!(&form.kind, SyntaxKind::Call { name } if name == "main") {
                    collect_source(form, file, &mut source);
                }
            }
        }
    }
    source
}

fn resolve_operation(
    tree: &ValidatedSourceTree,
    source: &SourceOperation,
    resolved: &ResolvedOperation,
    output: &mut Vec<ConversionInsertion>,
) -> SourceResult<()> {
    if source.file != resolved.file || source.name != resolved.name {
        return Err(diagnostic(
            tree,
            "LKJ-SRC-MIGRATION-RESOLUTION",
            "resolved numeric operation order does not match exact source",
        ));
    }
    let has_i64 = resolved.argument_types.contains(&Type::I64);
    let has_f64 = resolved.argument_types.contains(&Type::F64);
    if !has_i64 || !has_f64 {
        return Ok(());
    }
    if source.arguments.len() != resolved.argument_types.len() {
        return Err(diagnostic(
            tree,
            "LKJ-SRC-MIGRATION-RESOLUTION",
            "resolved numeric operation arity does not match exact source",
        ));
    }
    for (span, ty) in source.arguments.iter().zip(&resolved.argument_types) {
        if *ty == Type::I64 {
            output.push(ConversionInsertion {
                file: source.file,
                start: span.start().byte() as usize,
                end: span.end().byte() as usize,
            });
        }
    }
    Ok(())
}

pub(super) fn relevant(operation: Operation) -> Option<&'static str> {
    match operation {
        Operation::Add
        | Operation::Subtract
        | Operation::Multiply
        | Operation::Divide
        | Operation::Less
        | Operation::LessEqual
        | Operation::Greater
        | Operation::GreaterEqual => Some(operation.name()),
        _ => None,
    }
}

fn collect_source(node: &SourceNode, file: usize, output: &mut Vec<SourceOperation>) {
    if let SyntaxKind::Call { name } = &node.kind {
        if matches!(
            name.as_str(),
            "+" | "-" | "*" | "div" | "lt" | "lte" | "gt" | "gte"
        ) {
            output.push(SourceOperation {
                file,
                name: name.clone(),
                arguments: node.children.iter().map(|child| child.span).collect(),
            });
        }
    }
    for child in &node.children {
        collect_source(child, file, output);
    }
}
