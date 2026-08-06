use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, TypeUnavailableReason};

pub(super) fn check_hole<'a>(
    tree: &'a crate::source::ValidatedSourceTree,
    node: u64,
    declaration_key: &str,
    hole_identity: &str,
    expected_type: &str,
) -> Result<crate::semantic::operations::holes::site::HoleSite<'a>, ProtocolError> {
    let site = crate::semantic::operations::holes::site::find(tree, node)?;
    if site.declaration_key != declaration_key || site.local_identity != hole_identity {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "typed-hole declaration or stable local identity is stale",
        ));
    }
    check_expected(&site.expected, expected_type)?;
    Ok(site)
}

pub(super) fn check_target_expected(
    tree: &crate::source::ValidatedSourceTree,
    node: u64,
    declaration_key: &str,
    expected_type: &str,
) -> Result<(), ProtocolError> {
    let (owner, expected) =
        crate::semantic::operations::holes::site::expected_for_node(tree, node)?;
    if owner != declaration_key {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "insert_hole target declaration is stale",
        ));
    }
    check_expected(&expected, expected_type)
}

fn check_expected(
    expected: &Result<crate::hir::Type, TypeUnavailableReason>,
    requested: &str,
) -> Result<(), ProtocolError> {
    let actual = expected.as_ref().map_err(|reason| {
        error(
            ProtocolErrorCode::ValidationFailed,
            format!("typed-hole expected type unavailable: {reason:?}"),
        )
    })?;
    if crate::semantic::operations::holes::types::canonical(actual) != requested {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "typed-hole expected type precondition is stale",
        ));
    }
    Ok(())
}

pub(super) fn validate_identity(identity: &str) -> Result<(), ProtocolError> {
    if crate::source::is_source_identifier(identity) {
        Ok(())
    } else {
        Err(error(
            ProtocolErrorCode::InvalidOperation,
            "invalid typed-hole identity",
        ))
    }
}
