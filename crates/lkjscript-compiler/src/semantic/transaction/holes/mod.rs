mod apply;
mod preconditions;

pub(super) use apply::delete as apply_delete;
use preconditions::{check_hole, check_target_expected, validate_identity};

use crate::semantic::codec::error;
use crate::semantic::schema::{
    Expression, IdentityRelationKind, ProtocolError, ProtocolErrorCode, TransactionOperation,
};
use crate::semantic::transaction::ResolvedOperation;

pub(super) fn resolve(
    tree: &crate::source::ValidatedSourceTree,
    request: &TransactionOperation,
) -> Result<ResolvedOperation, ProtocolError> {
    match request {
        TransactionOperation::InsertHole {
            declaration_key,
            entity_fingerprint,
            node,
            node_fingerprint,
            hole_identity,
            goal,
            expected_type,
        } => {
            validate_identity(hole_identity)?;
            check_target_expected(tree, *node, declaration_key, expected_type)?;
            replace(
                tree,
                declaration_key,
                entity_fingerprint,
                *node,
                node_fingerprint,
                Expression::TypedHole {
                    identity: hole_identity.clone(),
                    goal: goal.clone(),
                },
                IdentityRelationKind::InsertedHole,
            )
        }
        TransactionOperation::FillHole {
            declaration_key,
            entity_fingerprint,
            node,
            node_fingerprint,
            hole_identity,
            expected_type,
            expression,
        } => {
            if matches!(expression, Expression::TypedHole { .. }) {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "fill_hole requires a complete concrete expression",
                ));
            }
            check_hole(tree, *node, declaration_key, hole_identity, expected_type)?;
            replace(
                tree,
                declaration_key,
                entity_fingerprint,
                *node,
                node_fingerprint,
                expression.clone(),
                IdentityRelationKind::FilledHole,
            )
        }
        TransactionOperation::RefineHole {
            declaration_key,
            entity_fingerprint,
            node,
            node_fingerprint,
            hole_identity,
            expected_type,
            goal,
        } => {
            check_hole(tree, *node, declaration_key, hole_identity, expected_type)?;
            replace(
                tree,
                declaration_key,
                entity_fingerprint,
                *node,
                node_fingerprint,
                Expression::TypedHole {
                    identity: hole_identity.clone(),
                    goal: goal.clone(),
                },
                IdentityRelationKind::RefinedHole,
            )
        }
        TransactionOperation::DeleteHole {
            declaration_key,
            entity_fingerprint,
            node,
            node_fingerprint,
            hole_identity,
            expected_type,
        } => {
            let site = check_hole(tree, *node, declaration_key, hole_identity, expected_type)?;
            if !crate::semantic::operations::holes::site::deletion_legal(&site) {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "delete_hole would not leave a structurally legal expression collection",
                ));
            }
            let resolved = super::replace::resolve(
                tree,
                declaration_key,
                entity_fingerprint,
                *node,
                node_fingerprint,
                &Expression::Unit {},
            )?;
            let ResolvedOperation::Replace {
                key, node, path, ..
            } = resolved
            else {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "hole target did not resolve",
                ));
            };
            Ok(ResolvedOperation::DeleteHole {
                key,
                node,
                owner: site.owner_node,
                path,
            })
        }
        _ => Err(error(
            ProtocolErrorCode::InvalidOperation,
            "operation is not a hole transaction",
        )),
    }
}

fn replace(
    tree: &crate::source::ValidatedSourceTree,
    declaration_key: &str,
    entity_fingerprint: &str,
    node: u64,
    node_fingerprint: &str,
    expression: Expression,
    relation: IdentityRelationKind,
) -> Result<ResolvedOperation, ProtocolError> {
    let resolved = super::replace::resolve(
        tree,
        declaration_key,
        entity_fingerprint,
        node,
        node_fingerprint,
        &expression,
    )?;
    let ResolvedOperation::Replace {
        key,
        node,
        path,
        replacement,
        ..
    } = resolved
    else {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "hole replacement did not resolve",
        ));
    };
    Ok(ResolvedOperation::Replace {
        key,
        node,
        path,
        replacement,
        relation,
    })
}
