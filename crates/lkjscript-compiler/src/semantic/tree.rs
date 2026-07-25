use crate::semantic::schema::{
    DeclarationRecord, Expression, NodeRecord, PositionRecord, SourceUnitRecord, SpanRecord,
};
use crate::source::{
    DeclarationSummary, NodeKind, NodeSummary, SourceNode, SourceSpan, SyntaxKind,
    ValidatedSourceTree,
};

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub(crate) fn fingerprint(node: &SourceNode) -> String {
    hex(&lkjscript_core::sha256(
        crate::source::format_node_identity(node).as_bytes(),
    ))
}

pub(crate) fn span_record(span: SourceSpan) -> SpanRecord {
    let start = span.start();
    let end = span.end();
    SpanRecord {
        start: PositionRecord {
            byte: start.byte(),
            line: start.line(),
            column: start.column(),
        },
        end: PositionRecord {
            byte: end.byte(),
            line: end.line(),
            column: end.column(),
        },
    }
}

pub(crate) fn expression(node: &SourceNode) -> Expression {
    match &node.kind {
        SyntaxKind::Unit => Expression::Unit,
        SyntaxKind::Bool { value } => Expression::Bool { value: *value },
        SyntaxKind::I64 { value } => Expression::I64 { value: *value },
        SyntaxKind::F64 { value } => Expression::F64 {
            value: crate::source::format_f64(*value),
        },
        SyntaxKind::Str { value } => Expression::String {
            value: value.clone(),
        },
        SyntaxKind::Symbol { name } => Expression::Symbol { name: name.clone() },
        SyntaxKind::Call { name } => Expression::Call {
            name: name.clone(),
            children: node.children.iter().map(expression).collect(),
        },
    }
}

pub(crate) fn source_units(tree: &ValidatedSourceTree) -> Vec<SourceUnitRecord> {
    let mut files: Vec<_> = tree.files().iter().collect();
    files.sort_by(|a, b| a.origin.logical_path.cmp(&b.origin.logical_path));
    files
        .into_iter()
        .map(|file| SourceUnitRecord {
            path: file.origin.logical_path.clone(),
            bytes: file.exact_source_len,
            sha256: hex(&file.exact_source_sha256),
        })
        .collect()
}

pub(crate) fn source_nodes(tree: &ValidatedSourceTree) -> Vec<&SourceNode> {
    let mut files: Vec<_> = tree.files().iter().collect();
    files.sort_by(|a, b| a.origin.logical_path.cmp(&b.origin.logical_path));
    let mut output = Vec::new();
    for file in files {
        for form in &file.syntax {
            flatten(form, &mut output);
        }
    }
    output
}

fn flatten<'a>(node: &'a SourceNode, output: &mut Vec<&'a SourceNode>) {
    output.push(node);
    for child in &node.children {
        flatten(child, output);
    }
}

pub(crate) fn declaration_record(
    tree: &ValidatedSourceTree,
    declaration: &DeclarationSummary,
) -> DeclarationRecord {
    let nodes = source_nodes(tree);
    let node = nodes.get(declaration.node().index() as usize);
    DeclarationRecord {
        key: declaration.key().to_hex(),
        identity: declaration.key().canonical_identity().to_string(),
        kind: declaration.kind().as_str().to_string(),
        name: declaration.name().to_string(),
        source: declaration.origin().logical_path().to_string(),
        span: span_record(declaration.span()),
        node: declaration.node().index(),
        fingerprint: node.map_or_else(String::new, |node| fingerprint(node)),
    }
}

pub(crate) fn containing_declaration<'a>(
    tree: &'a ValidatedSourceTree,
    node: &NodeSummary,
) -> Option<&'a DeclarationSummary> {
    let mut id = node.id();
    loop {
        if let Some(declaration) = tree
            .declarations()
            .iter()
            .find(|declaration| declaration.node() == id)
        {
            return Some(declaration);
        }
        let current = tree.node(id).ok()??;
        id = current.parent()?;
    }
}

pub(crate) fn node_record(tree: &ValidatedSourceTree, node: &NodeSummary) -> NodeRecord {
    let syntax = source_nodes(tree);
    NodeRecord {
        index: node.id().index(),
        kind: node_kind(node.kind()).to_string(),
        label: node.label().map(str::to_string),
        source: node.origin().logical_path().to_string(),
        span: span_record(node.span()),
        parent: node.parent().map(|id| id.index()),
        children: node.children().iter().map(|id| id.index()).collect(),
        declaration: containing_declaration(tree, node).map(|decl| decl.key().to_hex()),
        fingerprint: syntax
            .get(node.id().index() as usize)
            .map_or_else(String::new, |source| fingerprint(source)),
    }
}

fn node_kind(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::I64Literal => "i64_literal",
        NodeKind::F64Literal => "f64_literal",
        NodeKind::BoolLiteral => "bool_literal",
        NodeKind::UnitLiteral => "unit_literal",
        NodeKind::StringLiteral => "string_literal",
        NodeKind::Symbol => "symbol",
        NodeKind::Call => "call",
    }
}
