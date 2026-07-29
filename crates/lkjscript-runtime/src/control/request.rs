use lkjscript_contracts::ContractDigest;

use super::framing::{array, body, frame};
use super::{
    ApplicationInstallRequest, ControlError, ControlIdentity, ControlOperation, ControlRequest,
    SessionBackend,
};

const HEADER_BYTES: usize = 81;

pub fn encode(request: &ControlRequest) -> Result<Vec<u8>, ControlError> {
    let mut body = Vec::new();
    body.extend_from_slice(&request.identity.platform_revision.to_le_bytes());
    body.extend_from_slice(&request.identity.contract_digest.as_bytes());
    body.extend_from_slice(&request.request_id.to_le_bytes());
    body.extend_from_slice(&request.idempotency_id);
    body.push(request.operation.kind());
    encode_operation(&mut body, &request.operation)?;
    frame(body)
}

pub fn decode(frame_bytes: &[u8]) -> Result<ControlRequest, ControlError> {
    let body = body(frame_bytes)?;
    if body.len() < HEADER_BYTES {
        return Err(ControlError::Malformed("request length"));
    }
    let revision = u64::from_le_bytes(array(&body[..8])?);
    let contract = ContractDigest::from_bytes(array(&body[8..40])?);
    let request_id = u64::from_le_bytes(array(&body[40..48])?);
    let idempotency_id = array(&body[48..80])?;
    let operation = decode_operation(body[80], &body[81..])?;
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

fn encode_operation(bytes: &mut Vec<u8>, operation: &ControlOperation) -> Result<(), ControlError> {
    match operation {
        ControlOperation::Describe
        | ControlOperation::Status
        | ControlOperation::Shutdown
        | ControlOperation::ApplicationList
        | ControlOperation::SessionList => {}
        ControlOperation::ApplicationInstall(request) => encode_install(bytes, request)?,
        ControlOperation::ApplicationStart { application }
        | ControlOperation::ApplicationStop { application }
        | ControlOperation::ApplicationRestart { application }
        | ControlOperation::ApplicationRemove { application } => nonzero(bytes, *application)?,
        ControlOperation::ApplicationInvoke {
            application,
            arguments,
        } => {
            nonzero(bytes, *application)?;
            encode_arguments(bytes, arguments)?;
        }
        ControlOperation::SessionRegister {
            broker_instance,
            backend,
        } => {
            if *broker_instance == [0; 32] {
                return Err(ControlError::InvalidIdentity);
            }
            bytes.extend_from_slice(broker_instance);
            bytes.push(match backend {
                SessionBackend::None => 0,
            });
        }
        ControlOperation::SessionHeartbeat { session }
        | ControlOperation::SessionUnregister { session } => nonzero(bytes, *session)?,
    }
    Ok(())
}

fn decode_operation(tag: u8, bytes: &[u8]) -> Result<ControlOperation, ControlError> {
    let mut input = Input::new(bytes);
    let operation = match tag {
        1 => ControlOperation::Describe,
        2 => ControlOperation::Status,
        3 => ControlOperation::Shutdown,
        10 => ControlOperation::ApplicationInstall(decode_install(&mut input)?),
        11 => ControlOperation::ApplicationList,
        12 => ControlOperation::ApplicationStart {
            application: input.nonzero()?,
        },
        13 => ControlOperation::ApplicationStop {
            application: input.nonzero()?,
        },
        14 => ControlOperation::ApplicationRestart {
            application: input.nonzero()?,
        },
        15 => ControlOperation::ApplicationRemove {
            application: input.nonzero()?,
        },
        16 => ControlOperation::ApplicationInvoke {
            application: input.nonzero()?,
            arguments: decode_arguments(&mut input)?,
        },
        20 => {
            let broker_instance = input.array()?;
            if broker_instance == [0; 32] {
                return Err(ControlError::InvalidIdentity);
            }
            ControlOperation::SessionRegister {
                broker_instance,
                backend: match input.u8()? {
                    0 => SessionBackend::None,
                    _ => return Err(ControlError::Malformed("session backend")),
                },
            }
        }
        21 => ControlOperation::SessionList,
        22 => ControlOperation::SessionHeartbeat {
            session: input.nonzero()?,
        },
        23 => ControlOperation::SessionUnregister {
            session: input.nonzero()?,
        },
        _ => return Err(ControlError::Malformed("unknown operation")),
    };
    input.finish()?;
    Ok(operation)
}

include!("request_fields.rs");
