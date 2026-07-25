use super::fact_records::{available_reference, unavailable};
use crate::semantic::schema::{
    FactRecord, FactReference, FactSchema, ProducerStage, UnavailableReason,
};
use crate::source::{DeclarationKind, NodeKind, SourceNode, SyntaxKind, ValidatedSourceTree};

pub(super) fn binding_fact(
    tree: &ValidatedSourceTree,
    index: u32,
    node: &crate::source::NodeSummary,
    revision: &str,
) -> FactRecord {
    if !matches!(node.kind(), NodeKind::Call | NodeKind::Symbol) {
        return unavailable_fact(revision, UnavailableReason::NotApplicable);
    }
    let Some(name) = node.label() else {
        return unavailable_fact(revision, UnavailableReason::UnresolvedBinding);
    };
    let Some(owner) = crate::semantic::tree::containing_declaration(tree, node) else {
        return unavailable_fact(revision, UnavailableReason::StructuralPosition);
    };
    let nodes = crate::semantic::tree::source_nodes(tree);
    let Some(root) = nodes.get(owner.node().index() as usize) else {
        return unavailable_fact(revision, UnavailableReason::DerivedArtifactUnavailable);
    };
    let Ok(path) = crate::semantic::transaction::path_from_owner(tree, owner.node().index(), index)
    else {
        return unavailable_fact(revision, UnavailableReason::DerivedArtifactUnavailable);
    };
    if !crate::semantic::transaction::is_expression_path(root, &path) {
        return unavailable_fact(revision, UnavailableReason::StructuralPosition);
    }
    if lexical_binding_shadows(root, &path, name) {
        return unavailable_fact(revision, UnavailableReason::LexicalBindingMayShadow);
    }
    let Some(declaration) = tree.declarations().iter().find(|declaration| {
        declaration.kind() == DeclarationKind::Function && declaration.name() == name
    }) else {
        return unavailable_fact(revision, UnavailableReason::UnresolvedBinding);
    };
    available_reference(
        FactSchema::Binding,
        ProducerStage::SourceResolution,
        revision,
        FactReference::Declaration {
            key: declaration.key().to_hex(),
        },
    )
}

fn unavailable_fact(revision: &str, reason: UnavailableReason) -> FactRecord {
    unavailable(
        FactSchema::Binding,
        ProducerStage::SourceResolution,
        revision,
        reason,
    )
}

fn lexical_binding_shadows(root: &SourceNode, path: &[usize], target: &str) -> bool {
    let mut node = root;
    for child_index in path {
        if ancestor_introduces_name(node, *child_index, target) {
            return true;
        }
        let Some(child) = node.children.get(*child_index) else {
            return true;
        };
        node = child;
    }
    false
}

fn ancestor_introduces_name(node: &SourceNode, child_index: usize, target: &str) -> bool {
    let SyntaxKind::Call { name } = &node.kind else {
        return false;
    };
    match name.as_str() {
        "fn" if child_index + 1 == node.children.len() => node
            .children
            .iter()
            .find(|child| matches!(&child.kind, SyntaxKind::Call { name } if name == "params"))
            .is_some_and(|params| {
                params
                    .children
                    .iter()
                    .step_by(2)
                    .any(|parameter| source_name(parameter) == Some(target))
            }),
        "let" => node.children[..child_index.min(node.children.len())]
            .iter()
            .any(|binding| binding_name(binding) == Some(target)),
        "var" if child_index == 3 => {
            node.children
                .first()
                .and_then(|form| form.children.first())
                .and_then(source_name)
                == Some(target)
        }
        _ => false,
    }
}

fn binding_name(node: &SourceNode) -> Option<&str> {
    if !matches!(&node.kind, SyntaxKind::Call { name } if name == "bind") {
        return None;
    }
    node.children.first().and_then(source_name)
}

fn source_name(node: &SourceNode) -> Option<&str> {
    match &node.kind {
        SyntaxKind::Str { value } => Some(value),
        SyntaxKind::Symbol { name } => Some(name),
        _ => None,
    }
}
