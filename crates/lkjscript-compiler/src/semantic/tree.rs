mod identities;
mod projection;

use identities::enum_node_identity;
use projection::{projections, Projection};

use crate::semantic::schema::{
    DeclarationRecord, NodeRecord, PositionRecord, SemanticDeclarationKind, SemanticSubtreeRecord,
    SourceUnitRecord, SpanRecord, TriviaAttachment, TriviaRecord,
};
use crate::source::{
    DeclarationKind, DeclarationSummary, NodeSummary, SourceNode, SourceSpan, ValidatedSourceTree,
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

pub(crate) fn source_units(tree: &ValidatedSourceTree) -> Vec<SourceUnitRecord> {
    let mut files: Vec<_> = tree.files().iter().collect();
    files.sort_by(|a, b| a.origin.logical_path.cmp(&b.origin.logical_path));
    files
        .into_iter()
        .map(|file| SourceUnitRecord {
            path: file.origin.logical_path.clone(),
            identity: file.identity.to_hex(),
            bytes: file.exact_source_len,
            sha256: hex(&file.exact_source_sha256),
            trailing_trivia: vec![TriviaRecord {
                attachment: TriviaAttachment::SourceUnitTrailing,
                lines: file.trailing_trivia.clone(),
            }],
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
        kind: declaration_kind(declaration.kind()),
        name: declaration.name().to_string(),
        source: declaration.origin().logical_path().to_string(),
        span: span_record(declaration.span()),
        node: declaration.node().index(),
        fingerprint: node.map_or_else(String::new, |node| fingerprint(node)),
    }
}

fn declaration_kind(kind: DeclarationKind) -> SemanticDeclarationKind {
    match kind {
        DeclarationKind::Main => SemanticDeclarationKind::Main,
        DeclarationKind::Function => SemanticDeclarationKind::Function,
        DeclarationKind::Product => SemanticDeclarationKind::Product,
        DeclarationKind::Enum => SemanticDeclarationKind::Enum,
        DeclarationKind::Trait => SemanticDeclarationKind::MarkerTrait,
        DeclarationKind::Implementation => SemanticDeclarationKind::TraitImplementation,
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

pub(crate) fn node_records(tree: &ValidatedSourceTree) -> Vec<NodeRecord> {
    let source = source_nodes(tree);
    let projection = projections(tree);
    tree.nodes()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            Some(node_record_parts(
                tree,
                node,
                source.get(index)?,
                projection.get(index)?,
                &source,
            ))
        })
        .collect()
}

pub(crate) fn node_record(tree: &ValidatedSourceTree, node: &NodeSummary) -> NodeRecord {
    let index = node.id().index() as usize;
    let source = source_nodes(tree);
    let projection = projections(tree);
    node_record_parts(tree, node, source[index], &projection[index], &source)
}

fn node_record_parts(
    tree: &ValidatedSourceTree,
    node: &NodeSummary,
    source: &SourceNode,
    projection: &Projection,
    source_nodes: &[&SourceNode],
) -> NodeRecord {
    NodeRecord {
        index: node.id().index(),
        kind: projection.kind,
        value: projection.value.clone(),
        source: node.origin().logical_path().to_string(),
        span: span_record(node.span()),
        parent: node.parent().map(|id| id.index()),
        children: node.children().iter().map(|id| id.index()).collect(),
        declaration: containing_declaration(tree, node).map(|decl| decl.key().to_hex()),
        semantic_identity: enum_node_identity(tree, node, source, projection, source_nodes),
        fingerprint: fingerprint(source),
        trivia: projection.trivia.clone(),
    }
}

pub(crate) fn subtree_record(
    tree: &ValidatedSourceTree,
    root_index: u32,
) -> Option<SemanticSubtreeRecord> {
    let records = node_records(tree);
    build_subtree(&records, root_index)
}

fn build_subtree(records: &[NodeRecord], index: u32) -> Option<SemanticSubtreeRecord> {
    let node = records.get(index as usize)?.clone();
    let children = node
        .children
        .iter()
        .map(|child| build_subtree(records, *child))
        .collect::<Option<Vec<_>>>()?;
    Some(SemanticSubtreeRecord { node, children })
}
