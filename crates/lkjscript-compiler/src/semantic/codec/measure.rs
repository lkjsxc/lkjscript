mod operation;

use crate::semantic::schema::{
    ExpressionCounts, OperationRequest, ProtocolError, ProtocolErrorCode, Request,
};

pub(super) fn request(request: &Request) -> Result<(), ProtocolError> {
    let mut strings = 0;
    add(&mut strings, &[request.schema.len(), request.root.len()])?;
    let mut expression = ExpressionCounts::default();
    let operations = match &request.operation {
        OperationRequest::Snapshot {
            expected_repository_identity,
        } => {
            add(
                &mut strings,
                &[expected_repository_identity.as_ref().map_or(0, String::len)],
            )?;
            0
        }
        OperationRequest::ReadEntity {
            revision,
            declaration_key,
            entity_fingerprint,
        } => {
            add(
                &mut strings,
                &[
                    revision.len(),
                    declaration_key.len(),
                    entity_fingerprint.as_ref().map_or(0, String::len),
                ],
            )?;
            0
        }
        OperationRequest::QueryNode { revision, .. }
        | OperationRequest::HoleContext { revision, .. }
        | OperationRequest::LegalActions { revision, .. }
        | OperationRequest::Diagnostics { revision, .. } => {
            add(&mut strings, &[revision.len()])?;
            0
        }
        OperationRequest::ApplyTransaction {
            base_revision,
            file_preconditions,
            operations,
            ..
        } => {
            add(&mut strings, &[base_revision.len()])?;
            for file in file_preconditions {
                add(&mut strings, &[file.path.len(), file.sha256.len()])?;
            }
            for operation in operations {
                operation::measure(operation, &mut strings, &mut expression)?;
            }
            operations.len()
        }
    };
    if operations > super::MAX_OPERATIONS || expression.nodes > super::MAX_SCHEMA_NODES {
        return Err(super::error(
            ProtocolErrorCode::ResourceLimit,
            "operation or schema-node limit exceeded",
        ));
    }
    let decoded = strings
        .checked_add(expression.string_bytes)
        .ok_or_else(overflow)?;
    if expression.depth > super::MAX_JSON_DEPTH || decoded > super::MAX_STRING_BYTES {
        return Err(super::error(
            ProtocolErrorCode::ResourceLimit,
            "nesting or decoded-string limit exceeded",
        ));
    }
    Ok(())
}

pub(super) fn add(total: &mut u64, lengths: &[usize]) -> Result<(), ProtocolError> {
    for length in lengths {
        let length = u64::try_from(*length).map_err(|_| overflow())?;
        *total = total.checked_add(length).ok_or_else(overflow)?;
    }
    Ok(())
}

fn overflow() -> ProtocolError {
    super::error(
        ProtocolErrorCode::ResourceLimit,
        "decoded request charge overflow",
    )
}
