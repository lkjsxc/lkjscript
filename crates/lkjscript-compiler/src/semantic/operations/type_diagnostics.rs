use crate::semantic::schema::{DiagnosticCategory, DiagnosticCode, DiagnosticRecord};
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(super) fn declared_body_mismatch(tree: &ValidatedSourceTree) -> Option<DiagnosticRecord> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    for declaration in tree.declarations() {
        let Some(root) = nodes.get(declaration.node().index() as usize) else {
            continue;
        };
        let Some((signature, body)) = declaration_body(root) else {
            continue;
        };
        let Some(expected) = return_type(signature) else {
            continue;
        };
        let Some(actual) = literal_type(body) else {
            continue;
        };
        if expected != actual {
            return Some(super::diagnostic_records::node(
                DiagnosticCode::TypeMismatch,
                DiagnosticCategory::Type,
                tree,
                node_index(tree, declaration.node().index(), body)?,
                format!(
                    "declared return type {expected} does not exactly equal body type {actual}"
                ),
                Some(expected),
                Some(actual.to_string()),
            ));
        }
    }
    None
}

fn declaration_body(node: &SourceNode) -> Option<(&SourceNode, &SourceNode)> {
    let SyntaxKind::Call { name } = &node.kind else {
        return None;
    };
    match name.as_str() {
        "main" => Some((node.children.first()?, node.children.get(1)?)),
        "def" => {
            let function = node.children.get(1)?;
            let signature = function
                .children
                .iter()
                .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "sig"))?;
            Some((signature, function.children.last()?))
        }
        _ => None,
    }
}

fn return_type(signature: &SourceNode) -> Option<String> {
    let arrow = signature
        .children
        .iter()
        .position(|child| matches!(&child.kind, SyntaxKind::Symbol { name } if name == "->"))?;
    let atoms: Vec<_> = signature.children[arrow + 1..]
        .iter()
        .filter_map(|child| match &child.kind {
            SyntaxKind::Symbol { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    if atoms.is_empty() {
        None
    } else {
        Some(atoms.join(" "))
    }
}

fn literal_type(node: &SourceNode) -> Option<&'static str> {
    match &node.kind {
        SyntaxKind::I64 { .. } => Some("I64"),
        SyntaxKind::F64 { .. } => Some("F64"),
        SyntaxKind::Bool { .. } => Some("Bool"),
        SyntaxKind::Unit => Some("Unit"),
        SyntaxKind::Str { .. } => Some("Str"),
        SyntaxKind::Symbol { .. } | SyntaxKind::Call { .. } | SyntaxKind::EditionMarker => None,
    }
}

fn node_index(tree: &ValidatedSourceTree, root: u32, target: &SourceNode) -> Option<u32> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    nodes
        .iter()
        .enumerate()
        .skip(root as usize)
        .find(|(_, node)| std::ptr::eq(**node, target))
        .and_then(|(index, _)| u32::try_from(index).ok())
}
