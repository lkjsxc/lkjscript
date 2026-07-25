use crate::semantic::schema::{
    ExpressionCounts, OperationRequest, ProtocolError, ProtocolErrorCode, Request,
    TransactionOperation,
};

pub(crate) use super::response_codec::encode_response;

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_JSON_DEPTH: u32 = 64;
pub(crate) const MAX_STRING_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_SCHEMA_NODES: u64 = 65_536;
pub(crate) const MAX_OPERATIONS: usize = 64;
pub(crate) const MAX_WORK_UNITS: u64 = 1_000_000;
pub(crate) const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn decode_request(input: &[u8]) -> Result<Request, ProtocolError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(error(
            ProtocolErrorCode::ResourceLimit,
            format!("request bytes {} exceed {MAX_REQUEST_BYTES}", input.len()),
        ));
    }
    std::str::from_utf8(input).map_err(|_| {
        error(
            ProtocolErrorCode::InvalidJson,
            "request is not well-formed UTF-8",
        )
    })?;
    check_json_depth(input)?;
    let request: Request = serde_json::from_slice(input).map_err(|failure| {
        error(
            ProtocolErrorCode::InvalidJson,
            format!("strict JSON request rejected: {failure}"),
        )
    })?;
    if request.schema != super::SCHEMA {
        return Err(error(
            ProtocolErrorCode::InvalidSchema,
            format!("unknown schema {:?}", request.schema),
        ));
    }
    if request.version != super::VERSION {
        return Err(error(
            ProtocolErrorCode::UnsupportedVersion,
            format!("unsupported schema version {}", request.version),
        ));
    }
    super::charges::ProtocolLimits::for_profile(request.profile).check_request(input.len())?;
    measure_request(&request)?;
    Ok(request)
}

fn check_json_depth(input: &[u8]) -> Result<(), ProtocolError> {
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for byte in input {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match *byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > MAX_JSON_DEPTH {
                    return Err(error(
                        ProtocolErrorCode::ResourceLimit,
                        format!("JSON nesting exceeds {MAX_JSON_DEPTH}"),
                    ));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn measure_request(request: &Request) -> Result<(), ProtocolError> {
    let mut strings = request.schema.len() as u64 + request.root.len() as u64;
    let mut expression = ExpressionCounts::default();
    let operations = match &request.operation {
        OperationRequest::Snapshot {
            expected_repository_identity,
        } => {
            strings += expected_repository_identity
                .as_ref()
                .map_or(0, |s| s.len() as u64);
            0
        }
        OperationRequest::ReadEntity {
            revision,
            declaration_key,
            entity_fingerprint,
        } => {
            strings += revision.len() as u64 + declaration_key.len() as u64;
            strings += entity_fingerprint.as_ref().map_or(0, |s| s.len() as u64);
            0
        }
        OperationRequest::QueryNode { revision, .. }
        | OperationRequest::Diagnostics { revision, .. } => {
            strings += revision.len() as u64;
            0
        }
        OperationRequest::ApplyTransaction {
            base_revision,
            file_preconditions,
            operations,
            ..
        } => {
            strings += base_revision.len() as u64;
            for file in file_preconditions {
                strings += file.path.len() as u64 + file.sha256.len() as u64;
            }
            for operation in operations {
                match operation {
                    TransactionOperation::RenameDeclaration {
                        declaration_key,
                        entity_fingerprint,
                        new_name,
                    } => {
                        strings += (declaration_key.len()
                            + entity_fingerprint.len()
                            + new_name.len()) as u64
                    }
                    TransactionOperation::ReplaceExpression {
                        declaration_key,
                        entity_fingerprint,
                        node_fingerprint,
                        expression: replacement,
                        ..
                    } => {
                        strings += (declaration_key.len()
                            + entity_fingerprint.len()
                            + node_fingerprint.len()) as u64;
                        replacement.measure(1, &mut expression);
                    }
                }
            }
            operations.len()
        }
    };
    if operations > MAX_OPERATIONS || expression.nodes > MAX_SCHEMA_NODES {
        return Err(error(
            ProtocolErrorCode::ResourceLimit,
            "operation or schema-node limit exceeded",
        ));
    }
    if expression.depth > MAX_JSON_DEPTH || strings + expression.string_bytes > MAX_STRING_BYTES {
        return Err(error(
            ProtocolErrorCode::ResourceLimit,
            "nesting or decoded-string limit exceeded",
        ));
    }
    Ok(())
}

pub(crate) fn error(code: ProtocolErrorCode, message: impl Into<String>) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        diagnostic: None,
    }
}
