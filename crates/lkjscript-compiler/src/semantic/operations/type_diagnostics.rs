use crate::semantic::schema::{DiagnosticCategory, DiagnosticCode, DiagnosticRecord};
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};

pub(super) fn declared_body_mismatch(tree: &ValidatedSourceTree) -> Option<DiagnosticRecord> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    for declaration in tree.declarations() {
        let Some(root) = usize::try_from(declaration.node().index())
            .ok()
            .and_then(|index| nodes.get(index))
        else {
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
        "main" => Some((node.children.first()?, node.children.last()?)),
        "def" => {
            let function = node
                .children
                .iter()
                .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "fn"))?;
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
    let output = signature
        .children
        .iter()
        .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "output"))?;
    match output.children.as_slice() {
        [SourceNode {
            kind: SyntaxKind::Symbol { name },
            ..
        }] => Some(name.clone()),
        [SourceNode {
            kind: SyntaxKind::Unit,
            ..
        }] => Some("unit".into()),
        _ => None,
    }
}

fn literal_type(node: &SourceNode) -> Option<&'static str> {
    match &node.kind {
        SyntaxKind::I64 { .. } => Some("i64"),
        SyntaxKind::F64 { .. } => Some("f64"),
        SyntaxKind::Bool { .. } => Some("bool"),
        SyntaxKind::Unit => Some("unit"),
        SyntaxKind::Str { .. } => Some("string"),
        SyntaxKind::Bytes { .. } => Some("bytes"),
        SyntaxKind::Symbol { .. } | SyntaxKind::Call { .. } => None,
    }
}

fn node_index(tree: &ValidatedSourceTree, root: u64, target: &SourceNode) -> Option<u64> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    nodes
        .iter()
        .enumerate()
        .skip(usize::try_from(root).ok()?)
        .find(|(_, node)| std::ptr::eq(**node, target))
        .and_then(|(index, _)| u64::try_from(index).ok())
}
