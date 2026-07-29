use lkjscript_contracts::ContractDigest;

use super::framing::{array, body, frame};
use super::{ControlError, ControlIdentity, ControlOperation, ControlRequest};

const REQUEST_BYTES: usize = 81;

pub fn encode(request: &ControlRequest) -> Result<Vec<u8>, ControlError> {
    let mut body = Vec::with_capacity(REQUEST_BYTES);
    body.extend_from_slice(&request.identity.platform_revision.to_le_bytes());
    body.extend_from_slice(&request.identity.contract_digest.as_bytes());
    body.extend_from_slice(&request.request_id.to_le_bytes());
    body.extend_from_slice(&request.idempotency_id);
    body.push(match request.operation {
        ControlOperation::Describe => 1,
        ControlOperation::Status => 2,
        ControlOperation::Shutdown => 3,
    });
    frame(body)
}

pub fn decode(frame_bytes: &[u8]) -> Result<ControlRequest, ControlError> {
    let body = body(frame_bytes)?;
    if body.len() != REQUEST_BYTES {
        return Err(ControlError::Malformed("request length"));
    }
    let revision = u64::from_le_bytes(array(&body[..8])?);
    let contract = ContractDigest::from_bytes(array(&body[8..40])?);
    let request_id = u64::from_le_bytes(array(&body[40..48])?);
    let idempotency_id = array(&body[48..80])?;
    let operation = match body[80] {
        1 => ControlOperation::Describe,
        2 => ControlOperation::Status,
        3 => ControlOperation::Shutdown,
        _ => return Err(ControlError::Malformed("unknown operation")),
    };
    Ok(ControlRequest {
        identity: ControlIdentity {
            platform_revision: revision,
            contract_digest: contract,
        },
        request_id,
        idempotency_id,
        operation,
    })
}
