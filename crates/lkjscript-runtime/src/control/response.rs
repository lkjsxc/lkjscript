use lkjscript_contracts::ContractDigest;

use super::framing::{array, body, frame};
use super::{ControlError, ControlFailure, ControlResponse, ControlSuccess};

pub fn encode(response: &ControlResponse) -> Result<Vec<u8>, ControlError> {
    let mut body = Vec::new();
    body.extend_from_slice(&response.request_id.to_le_bytes());
    match &response.result {
        Ok(ControlSuccess::Description {
            platform_revision,
            contract_digest,
            product,
        }) => {
            if product.is_empty() || product.len() > 64 {
                return Err(ControlError::Malformed("product identity"));
            }
            body.push(1);
            body.extend_from_slice(&platform_revision.to_le_bytes());
            body.extend_from_slice(&contract_digest.as_bytes());
            put_u16(&mut body, product.len())?;
            body.extend_from_slice(product.as_bytes());
        }
        Ok(ControlSuccess::Status {
            coordinator,
            clean_shutdown,
            control_sequence,
            applications,
        }) => {
            body.push(2);
            body.extend_from_slice(&coordinator.to_le_bytes());
            body.push(u8::from(*clean_shutdown));
            body.extend_from_slice(&control_sequence.to_le_bytes());
            body.extend_from_slice(&applications.to_le_bytes());
        }
        Ok(ControlSuccess::ShutdownAccepted) => body.push(3),
        Err(failure) => encode_failure(&mut body, failure),
    }
    frame(body)
}

pub fn decode(frame_bytes: &[u8]) -> Result<ControlResponse, ControlError> {
    let body = body(frame_bytes)?;
    if body.len() < 9 {
        return Err(ControlError::Malformed("response length"));
    }
    let request_id = u64::from_le_bytes(array(&body[..8])?);
    let result = match body[8] {
        1 => decode_description(&body[9..]),
        2 => decode_status(&body[9..]),
        3 if body.len() == 9 => Ok(ControlSuccess::ShutdownAccepted),
        128 if body.len() == 9 => Err(ControlFailure::Unauthorized),
        129 => decode_stale(&body[9..]),
        130 if body.len() == 9 => Err(ControlFailure::ContractMismatch),
        131 if body.len() == 9 => Err(ControlFailure::ReplayConflict),
        132 if body.len() == 9 => Err(ControlFailure::Malformed),
        133 if body.len() == 9 => Err(ControlFailure::Internal),
        _ => return Err(ControlError::Malformed("response result")),
    };
    Ok(ControlResponse { request_id, result })
}

fn decode_description(bytes: &[u8]) -> Result<ControlSuccess, ControlFailure> {
    if bytes.len() < 42 {
        return Err(ControlFailure::Malformed);
    }
    let platform_revision = u64::from_le_bytes(array_failure(&bytes[..8])?);
    let contract_digest = ContractDigest::from_bytes(array_failure(&bytes[8..40])?);
    let length = u16::from_le_bytes(array_failure(&bytes[40..42])?) as usize;
    if bytes.len() != 42 + length || length == 0 || length > 64 {
        return Err(ControlFailure::Malformed);
    }
    let product = std::str::from_utf8(&bytes[42..])
        .map_err(|_| ControlFailure::Malformed)?
        .to_string();
    Ok(ControlSuccess::Description {
        platform_revision,
        contract_digest,
        product,
    })
}

fn decode_status(bytes: &[u8]) -> Result<ControlSuccess, ControlFailure> {
    if bytes.len() != 21 || bytes[8] > 1 {
        return Err(ControlFailure::Malformed);
    }
    Ok(ControlSuccess::Status {
        coordinator: u64::from_le_bytes(array_failure(&bytes[..8])?),
        clean_shutdown: bytes[8] == 1,
        control_sequence: u64::from_le_bytes(array_failure(&bytes[9..17])?),
        applications: u32::from_le_bytes(array_failure(&bytes[17..21])?),
    })
}

fn decode_stale(bytes: &[u8]) -> Result<ControlSuccess, ControlFailure> {
    if bytes.len() != 16 {
        return Err(ControlFailure::Malformed);
    }
    Err(ControlFailure::StaleRevision {
        expected: u64::from_le_bytes(array_failure(&bytes[..8])?),
        found: u64::from_le_bytes(array_failure(&bytes[8..16])?),
    })
}

fn encode_failure(bytes: &mut Vec<u8>, failure: &ControlFailure) {
    match failure {
        ControlFailure::Unauthorized => bytes.push(128),
        ControlFailure::StaleRevision { expected, found } => {
            bytes.push(129);
            bytes.extend_from_slice(&expected.to_le_bytes());
            bytes.extend_from_slice(&found.to_le_bytes());
        }
        ControlFailure::ContractMismatch => bytes.push(130),
        ControlFailure::ReplayConflict => bytes.push(131),
        ControlFailure::Malformed => bytes.push(132),
        ControlFailure::Internal => bytes.push(133),
    }
}

fn put_u16(bytes: &mut Vec<u8>, value: usize) -> Result<(), ControlError> {
    bytes.extend_from_slice(
        &u16::try_from(value)
            .map_err(|_| ControlError::Oversized)?
            .to_le_bytes(),
    );
    Ok(())
}

fn array_failure<const N: usize>(bytes: &[u8]) -> Result<[u8; N], ControlFailure> {
    bytes.try_into().map_err(|_| ControlFailure::Malformed)
}
