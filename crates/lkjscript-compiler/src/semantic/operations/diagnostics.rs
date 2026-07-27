use std::collections::{HashMap, HashSet};

use crate::semantic::schema::{DiagnosticCategory, DiagnosticCode, DiagnosticRecord};
use crate::source::{DeclarationKind, SourceNode, SyntaxKind, ValidatedSourceTree};

pub(crate) use super::diagnostic_records::{source_failure, stale};

pub(crate) fn collect(tree: &ValidatedSourceTree, include_hir: bool) -> Vec<DiagnosticRecord> {
    if !include_hir || crate::analyze::analyze_module_program(tree).is_ok() {
        return Vec::new();
    }
    let mut globals = HashSet::new();
    let mut arities = HashMap::new();
    for declaration in tree.declarations() {
        if declaration.kind() == DeclarationKind::Function {
            globals.insert(declaration.name().to_string());
        }
    }
    let source_nodes = crate::semantic::tree::source_nodes(tree);
    for node in &source_nodes {
        if let Some((name, arity)) = function_arity(node) {
            arities.insert(name, arity);
        }
    }
    let mut names_by_declaration = HashMap::new();
    for declaration in tree.declarations() {
        let mut names = globals.clone();
        if let Ok(index) = usize::try_from(declaration.node().index()) {
            if let Some(root) = source_nodes.get(index) {
                collect_local_names(root, &mut names);
            }
        }
        names_by_declaration.insert(declaration.key().to_hex(), names);
    }
    let mut diagnostics = Vec::new();
    for (source_index, node) in source_nodes.iter().enumerate() {
        let SyntaxKind::Call { name } = &node.kind else {
            continue;
        };
        if registered_form(name) || crate::hir::Operation::from_name(name).is_some() {
            continue;
        }
        let index = u32::try_from(source_index).unwrap_or(u32::MAX);
        let known = tree
            .nodes()
            .get(source_index)
            .and_then(|summary| crate::semantic::tree::containing_declaration(tree, summary))
            .and_then(|declaration| names_by_declaration.get(&declaration.key().to_hex()));
        if !known.is_some_and(|names| names.contains(name)) {
            diagnostics.push(super::diagnostic_records::node(
                DiagnosticCode::UnknownName,
                DiagnosticCategory::NameResolution,
                tree,
                index,
                format!("unknown call {name}"),
                None,
                None,
            ));
        } else if let Some(expected) = arities.get(name) {
            if *expected != node.children.len() {
                diagnostics.push(super::diagnostic_records::node(
                    DiagnosticCode::CallArity,
                    DiagnosticCategory::Call,
                    tree,
                    index,
                    format!(
                        "{name}: expected {expected} args, got {}",
                        node.children.len()
                    ),
                    Some(expected.to_string()),
                    Some(node.children.len().to_string()),
                ));
            }
        }
    }
    if diagnostics.is_empty() {
        if let Some(mismatch) = super::type_diagnostics::declared_body_mismatch(tree) {
            diagnostics.push(mismatch);
        } else {
            let index = tree
                .declarations()
                .first()
                .map_or(0, |decl| decl.node().index());
            diagnostics.push(super::diagnostic_records::node(
                DiagnosticCode::TypeMismatch,
                DiagnosticCategory::Type,
                tree,
                index,
                "HIR validation rejected the source under its declared type contract".to_string(),
                None,
                None,
            ));
        }
    }
    diagnostics.sort_by_key(|diagnostic| {
        (
            code_rank(diagnostic.code),
            diagnostic.primary_source.clone(),
            diagnostic.primary_span.start.byte,
        )
    });
    diagnostics
}

fn registered_form(name: &str) -> bool {
    lkjscript_contracts::CONTEXTUAL_FORM_NAMES.contains(&name)
        || lkjscript_contracts::TYPE_CONSTRUCTOR_NAMES.contains(&name)
}

fn collect_local_names(node: &SourceNode, known: &mut HashSet<String>) {
    if matches!(&node.kind, SyntaxKind::Call { name } if name == "name") {
        if let Some(child) = node.children.first() {
            match &child.kind {
                SyntaxKind::Str { value } => {
                    known.insert(value.clone());
                }
                SyntaxKind::Symbol { name } => {
                    known.insert(name.clone());
                }
                _ => {}
            }
        }
    }
    for child in &node.children {
        collect_local_names(child, known);
    }
}

fn function_arity(node: &SourceNode) -> Option<(String, usize)> {
    if !matches!(&node.kind, SyntaxKind::Call { name } if name == "def") {
        return None;
    }
    let name = node.children.first()?.children.first()?;
    let name = match &name.kind {
        SyntaxKind::Str { value } => value.clone(),
        _ => return None,
    };
    let function = node.children.get(1)?;
    let params = function
        .children
        .iter()
        .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "params"))?;
    Some((name, params.children.len() / 2))
}

fn code_rank(code: DiagnosticCode) -> u8 {
    match code {
        DiagnosticCode::UnmatchedMarker => 0,
        DiagnosticCode::DuplicateDeclaration => 1,
        DiagnosticCode::UnknownName => 2,
        DiagnosticCode::CallArity => 3,
        DiagnosticCode::TypeMismatch => 4,
        DiagnosticCode::StaleEdit => 5,
    }
}
