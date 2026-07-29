use lkjscript_contracts::ContractDigest;

use super::framing::{array, body, frame};
use super::{
    ControlError, ControlFailure, ControlResponse, ControlSuccess, ControlledApplication,
    ControlledApplicationState,
};

const MAX_APPLICATIONS: usize = 1_024;
const MAX_NAME_BYTES: usize = 64;
const MAX_REJECTION_BYTES: usize = 4_096;
const MAX_INVOKE_OUTPUT_BYTES: usize = 16 * 1024;
const MAX_OUTCOME_BYTES: usize = 44 * 1024;

pub fn encode(response: &ControlResponse) -> Result<Vec<u8>, ControlError> {
    let mut body = Vec::new();
    body.extend_from_slice(&response.request_id.to_le_bytes());
    match &response.result {
        Ok(ControlSuccess::Description {
            platform_revision,
            contract_digest,
            product,
        }) => encode_description(&mut body, *platform_revision, *contract_digest, product)?,
        Ok(ControlSuccess::Status {
            coordinator,
            clean_shutdown,
            control_sequence,
            applications,
        }) => encode_status(
            &mut body,
            *coordinator,
            *clean_shutdown,
            *control_sequence,
            *applications,
        ),
        Ok(ControlSuccess::ShutdownAccepted) => body.push(3),
        Ok(ControlSuccess::Application(application)) => {
            body.push(10);
            encode_application(&mut body, application)?;
        }
        Ok(ControlSuccess::Applications(applications)) => {
            if applications.len() > MAX_APPLICATIONS {
                return Err(ControlError::Oversized);
            }
            body.push(11);
            put_u16(&mut body, applications.len())?;
            for application in applications {
                encode_application(&mut body, application)?;
            }
        }
        Ok(ControlSuccess::ApplicationRemoved { application }) => {
            body.push(12);
            put_nonzero(&mut body, *application)?;
        }
        Ok(ControlSuccess::ApplicationInvoked {
            application,
            outcome,
            output,
        }) => encode_invoked(&mut body, *application, outcome, output)?,
        Err(failure) => encode_failure(&mut body, failure)?,
    }
    frame(body)
}

pub fn decode(frame_bytes: &[u8]) -> Result<ControlResponse, ControlError> {
    let body = body(frame_bytes)?;
    if body.len() < 9 {
        return Err(ControlError::Malformed("response length"));
    }
    let request_id = u64::from_le_bytes(array(&body[..8])?);
    let mut input = ResponseInput::new(&body[9..]);
    let result = match body[8] {
        1 => decode_description(&mut input),
        2 => decode_status(&mut input),
        3 => Ok(ControlSuccess::ShutdownAccepted),
        10 => Ok(ControlSuccess::Application(decode_application(&mut input)?)),
        11 => decode_applications(&mut input),
        12 => Ok(ControlSuccess::ApplicationRemoved {
            application: input.nonzero()?,
        }),
        13 => decode_invoked(&mut input),
        128 => Err(ControlFailure::Unauthorized),
        129 => decode_stale(&mut input),
        130 => Err(ControlFailure::ContractMismatch),
        131 => Err(ControlFailure::ReplayConflict),
        132 => Err(ControlFailure::Malformed),
        133 => Err(ControlFailure::Internal),
        134 => Err(ControlFailure::NotFound),
        135 => Err(ControlFailure::Rejected(input.text(MAX_REJECTION_BYTES)?)),
        _ => return Err(ControlError::Malformed("response result")),
    };
    input.finish()?;
    Ok(ControlResponse { request_id, result })
}

include!("response_base.rs");
include!("response_application.rs");
include!("response_input.rs");
