use crate::source::{NodeKind, SourceResult, SyntaxKind, ValidatedSourceTree};

use super::{diagnostic, EditionMigrationDeclarationIdentity, EditionMigrationNodeIdentity};

pub(super) fn declaration_relations(
    old: &ValidatedSourceTree,
    new: &ValidatedSourceTree,
) -> SourceResult<Vec<EditionMigrationDeclarationIdentity>> {
    old.declarations()
        .iter()
        .map(|before| {
            let after = new
                .declarations()
                .iter()
                .find(|after| {
                    after.kind() == before.kind()
                        && after.name() == before.name()
                        && after.origin() == before.origin()
                })
                .ok_or_else(|| {
                    diagnostic(old, "LKJ-SRC-MIGRATION-IDENTITY", "declaration disappeared")
                })?;
            Ok(EditionMigrationDeclarationIdentity {
                old_key: before.key().clone(),
                new_key: after.key().clone(),
                old_node: before.node(),
                new_node: after.node(),
            })
        })
        .collect()
}

pub(super) fn node_relations(
    old: &ValidatedSourceTree,
    new: &ValidatedSourceTree,
) -> SourceResult<Vec<EditionMigrationNodeIdentity>> {
    let mut relations = Vec::with_capacity(old.nodes().len());
    for old_file in old.files() {
        let new_file = new
            .files()
            .iter()
            .find(|file| file.origin == old_file.origin)
            .ok_or_else(|| diagnostic(old, "LKJ-SRC-MIGRATION-IDENTITY", "source disappeared"))?;
        let new_forms: Vec<_> = new_file
            .syntax
            .iter()
            .filter(|node| !matches!(node.kind, SyntaxKind::EditionMarker))
            .collect();
        if old_file.syntax.len() != new_forms.len() {
            return Err(diagnostic(
                old,
                "LKJ-SRC-MIGRATION-IDENTITY",
                "migration changed top-level declaration shape",
            ));
        }
        for (before, after) in old_file.syntax.iter().zip(new_forms) {
            relate_node(old, new, before, after, &old_file.origin, &mut relations)?;
        }
    }
    Ok(relations)
}

fn relate_node(
    old: &ValidatedSourceTree,
    new: &ValidatedSourceTree,
    before: &crate::source::SourceNode,
    mut after: &crate::source::SourceNode,
    origin: &crate::source::SourceOrigin,
    output: &mut Vec<EditionMigrationNodeIdentity>,
) -> SourceResult<()> {
    while !same_node_shape(before, after) {
        if matches!(&after.kind, SyntaxKind::Call { name } if name == "f64-from-i64-rounded")
            && after.children.len() == 1
        {
            after = &after.children[0];
        } else {
            return Err(diagnostic(
                old,
                "LKJ-SRC-MIGRATION-IDENTITY",
                "migration changed an existing node shape",
            ));
        }
    }
    output.push(EditionMigrationNodeIdentity {
        old: find_node(old, origin, before)?,
        new: find_node(new, origin, after)?,
    });
    if before.children.len() != after.children.len() {
        return Err(diagnostic(
            old,
            "LKJ-SRC-MIGRATION-IDENTITY",
            "migration changed an existing node arity",
        ));
    }
    for (old_child, new_child) in before.children.iter().zip(&after.children) {
        relate_node(old, new, old_child, new_child, origin, output)?;
    }
    Ok(())
}

fn same_node_shape(left: &crate::source::SourceNode, right: &crate::source::SourceNode) -> bool {
    match (&left.kind, &right.kind) {
        (SyntaxKind::I64 { value: a }, SyntaxKind::I64 { value: b }) => a == b,
        (SyntaxKind::F64 { value: a }, SyntaxKind::F64 { value: b }) => a.to_bits() == b.to_bits(),
        (SyntaxKind::Bool { value: a }, SyntaxKind::Bool { value: b }) => a == b,
        (SyntaxKind::Unit, SyntaxKind::Unit) => true,
        (SyntaxKind::Str { value: a }, SyntaxKind::Str { value: b })
        | (SyntaxKind::Symbol { name: a }, SyntaxKind::Symbol { name: b })
        | (SyntaxKind::Call { name: a }, SyntaxKind::Call { name: b }) => a == b,
        (SyntaxKind::EditionMarker, SyntaxKind::EditionMarker) => true,
        _ => false,
    }
}

fn find_node(
    tree: &ValidatedSourceTree,
    origin: &crate::source::SourceOrigin,
    node: &crate::source::SourceNode,
) -> SourceResult<crate::source::NodeId> {
    let kind = node_kind(node);
    tree.nodes()
        .iter()
        .find(|summary| {
            summary.origin() == origin && summary.span() == node.span && summary.kind() == kind
        })
        .map(|summary| summary.id())
        .ok_or_else(|| {
            diagnostic(
                tree,
                "LKJ-SRC-MIGRATION-IDENTITY",
                "node identity disappeared",
            )
        })
}

fn node_kind(node: &crate::source::SourceNode) -> NodeKind {
    match &node.kind {
        SyntaxKind::I64 { .. } => NodeKind::I64Literal,
        SyntaxKind::F64 { .. } => NodeKind::F64Literal,
        SyntaxKind::Bool { .. } => NodeKind::BoolLiteral,
        SyntaxKind::Unit => NodeKind::UnitLiteral,
        SyntaxKind::Str { .. } => NodeKind::StringLiteral,
        SyntaxKind::Symbol { .. } => NodeKind::Symbol,
        SyntaxKind::Call { .. } => NodeKind::Call,
        SyntaxKind::EditionMarker => NodeKind::EditionMarker,
    }
}
