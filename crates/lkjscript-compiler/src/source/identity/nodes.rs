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
            let id = flatten_node(form, &file.origin, revision, &mut nodes)?;
            top_ids.insert((*file_index, form_index), id);
        }
    }
    Ok((nodes, top_ids))
}

fn flatten_node(
    root: &SourceNode,
    origin: &SourceOrigin,
    revision: RevisionId,
    output: &mut Vec<NodeSummary>,
) -> SourceResult<NodeId> {
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| allocation_error(origin, root.span, "source identity work stack"))?;
    work.push((root, None));
    let mut root_id = None;

    while let Some((node, parent)) = work.pop() {
        let raw = u32::try_from(output.len()).map_err(|_| {
            SourceDiagnostic::generic(origin.clone(), "too many source nodes for NodeId")
        })?;
        let id = NodeId {
            revision,
            index: raw,
        };
        if root_id.is_none() {
            root_id = Some(id);
        }
        let (kind, label) = node_label(node);
        output
            .try_reserve(1)
            .map_err(|_| allocation_error(origin, node.span, "source identity nodes"))?;
        output.push(NodeSummary {
            id,
            kind,
            label,
            origin: origin.clone(),
            span: node.span,
            parent,
            children: Vec::new(),
        });
        if let Some(parent) = parent {
            let index = usize::try_from(parent.index).map_err(|_| {
                SourceDiagnostic::generic(origin.clone(), "source parent identity exceeds usize")
            })?;
            let summary = output.get_mut(index).ok_or_else(|| {
                SourceDiagnostic::generic(origin.clone(), "source parent identity is missing")
            })?;
            summary
                .children
                .try_reserve(1)
                .map_err(|_| allocation_error(origin, node.span, "source child identities"))?;
            summary.children.push(id);
        }
        work.try_reserve(node.children.len())
            .map_err(|_| allocation_error(origin, node.span, "source identity work stack"))?;
        work.extend(node.children.iter().rev().map(|child| (child, Some(id))));
    }

    root_id.ok_or_else(|| SourceDiagnostic::generic(origin.clone(), "source root is missing"))
}

fn node_label(node: &SourceNode) -> (NodeKind, Option<String>) {
    match &node.kind {
        SyntaxKind::I64 { value } => (NodeKind::I64Literal, Some(value.to_string())),
        SyntaxKind::F64 { value } => (NodeKind::F64Literal, Some(format::format_f64(*value))),
        SyntaxKind::Bool { value } => (NodeKind::BoolLiteral, Some(value.to_string())),
        SyntaxKind::Unit => (NodeKind::UnitLiteral, None),
        SyntaxKind::Str { value } => (NodeKind::StringLiteral, Some(value.clone())),
        SyntaxKind::Bytes { value } => (
            NodeKind::BytesLiteral,
            Some(value.iter().map(|byte| format!("{byte:02x}")).collect()),
        ),
        SyntaxKind::Symbol { name } => (NodeKind::Symbol, Some(name.clone())),
        SyntaxKind::Call { name } => (NodeKind::Call, Some(name.clone())),
    }
}

fn allocation_error(
    origin: &SourceOrigin,
    span: crate::source::SourceSpan,
    context: &str,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-HOST",
        crate::source::DiagnosticCategory::SourceLoading,
        format!("host could not reserve memory for {context}"),
        origin.clone(),
        span,
    )
}
