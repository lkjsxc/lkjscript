use std::collections::HashMap;

use crate::source::{
    format, NodeId, NodeKind, NodeSummary, RevisionId, SourceDiagnostic, SourceFile, SourceNode,
    SourceOrigin, SourceResult, SyntaxKind,
};

type TopNodeIds = HashMap<(usize, usize), NodeId>;

pub(crate) fn flatten_files(
    files: &[SourceFile],
    ordered: &[usize],
    revision: RevisionId,
) -> SourceResult<(Vec<NodeSummary>, TopNodeIds)> {
    let mut nodes = Vec::new();
    let mut top_ids = HashMap::new();
    for file_index in ordered {
        let file = &files[*file_index];
        for (form_index, form) in file.syntax.iter().enumerate() {
            let id = flatten_node(form, &file.origin, revision, None, &mut nodes)?;
            top_ids.insert((*file_index, form_index), id);
        }
    }
    Ok((nodes, top_ids))
}

fn flatten_node(
    node: &SourceNode,
    origin: &SourceOrigin,
    revision: RevisionId,
    parent: Option<NodeId>,
    output: &mut Vec<NodeSummary>,
) -> SourceResult<NodeId> {
    let raw = u32::try_from(output.len()).map_err(|_| {
        SourceDiagnostic::generic(origin.clone(), "too many source nodes for NodeId")
    })?;
    let id = NodeId {
        revision,
        index: raw,
    };
    let (kind, label) = match &node.kind {
        SyntaxKind::I64 { value } => (NodeKind::I64Literal, Some(value.to_string())),
        SyntaxKind::F64 { value } => (NodeKind::F64Literal, Some(format::format_f64(*value))),
        SyntaxKind::Bool { value } => (NodeKind::BoolLiteral, Some(value.to_string())),
        SyntaxKind::Unit => (NodeKind::UnitLiteral, None),
        SyntaxKind::Str { value } => (NodeKind::StringLiteral, Some(value.clone())),
        SyntaxKind::Symbol { name } => (NodeKind::Symbol, Some(name.clone())),
        SyntaxKind::Call { name } => (NodeKind::Call, Some(name.clone())),
        SyntaxKind::EditionMarker => (NodeKind::EditionMarker, Some("2".into())),
    };
    output.push(NodeSummary {
        id,
        kind,
        label,
        origin: origin.clone(),
        span: node.span,
        parent,
        children: Vec::new(),
    });
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        children.push(flatten_node(child, origin, revision, Some(id), output)?);
    }
    if let Some(summary) = output.get_mut(raw as usize) {
        summary.children = children;
    }
    Ok(id)
}
