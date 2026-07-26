mod witness;
mod witness_types;

use witness::scoped_witness;
pub(super) use witness::witness;
pub(super) use witness_types::type_expression;

use std::collections::BTreeSet;

use lkjscript_core::Limits;

use crate::semantic::schema::Expression;
use crate::source::{SourceFile, ValidatedSourceTree};

pub(crate) fn source_holes(tree: &ValidatedSourceTree) -> Result<(), String> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    let mut identities = BTreeSet::new();
    for (index, node) in nodes.iter().enumerate() {
        if !super::types::call_is(node, "hole") {
            continue;
        }
        let index = u32::try_from(index).map_err(|_| "hole NodeId overflow")?;
        let site = super::site::find(tree, index).map_err(|failure| failure.message)?;
        let identity = format!("{}:{}", site.declaration_key, site.local_identity);
        if !identities.insert(identity) {
            return Err("duplicate declaration-local typed-hole identity".into());
        }
    }
    Ok(())
}

pub(super) fn checker_accepts(site: &super::site::HoleSite<'_>, expression: &Expression) -> bool {
    completed_tree(site.tree, Some((site.node, expression.clone())))
        .and_then(|tree| crate::analyze::analyze_program(&tree).map_err(|error| error.to_string()))
        .is_ok()
}

pub(super) fn completed_program(
    site: &super::site::HoleSite<'_>,
) -> Result<crate::hir::Program, String> {
    let tree = completed_tree(site.tree, None)?;
    crate::analyze::analyze_program(&tree).map_err(|failure| failure.to_string())
}

pub(crate) fn validate_incomplete(tree: &ValidatedSourceTree) -> Result<(), String> {
    source_holes(tree)?;
    let completed = completed_tree(tree, None)?;
    crate::analyze::analyze_program(&completed)
        .map(|_| ())
        .map_err(|failure| failure.to_string())
}

pub(super) fn completed_tree(
    tree: &ValidatedSourceTree,
    target: Option<(u32, Expression)>,
) -> Result<ValidatedSourceTree, String> {
    let mut files = tree.files().to_vec();
    let nodes = crate::semantic::tree::source_nodes(tree);
    let mut replacements = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if !super::types::call_is(node, "hole") {
            continue;
        }
        let index = u32::try_from(index).map_err(|_| "hole NodeId overflow")?;
        let expression = if target.as_ref().is_some_and(|(node, _)| *node == index) {
            target
                .as_ref()
                .map_or_else(|| Expression::Unit {}, |(_, expression)| expression.clone())
        } else {
            let site = super::site::find(tree, index).map_err(|failure| failure.message)?;
            let ty = site
                .expected
                .as_ref()
                .map_err(|reason| format!("hole expected type unavailable: {reason:?}"))?;
            witness(tree, ty, 0)
                .or_else(|| scoped_witness(&site, ty))
                .ok_or_else(|| {
                    format!(
                        "no bounded validation witness for {}",
                        super::types::canonical(ty)
                    )
                })?
        };
        replacements.push((index, expression));
    }
    replacements.sort_by_key(|(index, _)| std::cmp::Reverse(*index));
    for (index, expression) in replacements {
        if !expression.supports_edition(tree.edition()) {
            return Err("hole expression requires Edition 2".into());
        }
        let target = crate::semantic::transaction::node_mut(&mut files, index)
            .ok_or("hole node disappeared during validation")?;
        *target = expression.to_source(target.span)?;
    }
    rebuild(tree, &files)
}

fn rebuild(
    tree: &ValidatedSourceTree,
    files: &[SourceFile],
) -> Result<ValidatedSourceTree, String> {
    let sources: Vec<_> = files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                file.origin.clone(),
                crate::source::format_file(file),
            )
        })
        .collect();
    crate::source::rebuild_staged_sources(
        &sources,
        tree.root_path().to_path_buf(),
        tree.root_origin().clone(),
        &Limits::default(),
    )
    .map_err(|failure| failure.render_human())
}
