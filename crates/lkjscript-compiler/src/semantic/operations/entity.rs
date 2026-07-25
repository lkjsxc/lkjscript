use crate::semantic::codec::error;
use crate::semantic::schema::{EntityRecord, ProtocolError, ProtocolErrorCode};
use crate::source::{DeclarationSummary, ValidatedSourceTree};

pub(crate) fn find<'a>(
    tree: &'a ValidatedSourceTree,
    key: &str,
) -> Result<&'a DeclarationSummary, ProtocolError> {
    tree.declarations()
        .iter()
        .find(|declaration| declaration.key().to_hex() == key)
        .ok_or_else(|| {
            error(
                ProtocolErrorCode::UnknownDeclaration,
                format!("unknown exact declaration key {key:?}"),
            )
        })
}

pub(crate) fn read(
    tree: &ValidatedSourceTree,
    key: &str,
    expected: Option<&str>,
) -> Result<EntityRecord, ProtocolError> {
    let declaration = find(tree, key)?;
    let syntax = crate::semantic::tree::source_nodes(tree);
    let source = syntax
        .get(declaration.node().index() as usize)
        .ok_or_else(|| {
            error(
                ProtocolErrorCode::UnknownNode,
                "declaration node is unavailable",
            )
        })?;
    let fingerprint = crate::semantic::tree::fingerprint(source);
    if expected.is_some_and(|value| value != fingerprint) {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "entity fingerprint precondition does not match",
        ));
    }
    let descendants = tree
        .nodes()
        .iter()
        .filter(|node| {
            crate::semantic::tree::containing_declaration(tree, node)
                .is_some_and(|owner| owner.key() == declaration.key())
        })
        .map(|node| crate::semantic::tree::node_record(tree, node))
        .collect();
    Ok(EntityRecord {
        declaration: crate::semantic::tree::declaration_record(tree, declaration),
        source: declaration.origin().logical_path().to_string(),
        fingerprint,
        canonical_subtree: crate::source::format_node_source(source),
        subtree: crate::semantic::tree::expression(source),
        descendants,
        derived_summaries: vec!["query_node:lkjscript.agent-foundation:1".to_string()],
    })
}
