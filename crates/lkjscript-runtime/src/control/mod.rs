use std::fmt;

use lkjscript_contracts::{
    current_contracts, ContractDigest, RegisteredContract, PLATFORM_REVISION,
};
use lkjscript_host::{HostError, LocalPrincipal};

mod framing;
mod model;
mod request;
mod response;
#[cfg(test)]
mod tests;
#[cfg(target_os = "linux")]
mod unix;

pub use model::*;
pub use request::{decode as decode_request_frame, encode as encode_request_frame};
pub use response::{decode as decode_response_frame, encode as encode_response_frame};
#[cfg(target_os = "linux")]
pub use unix::{UnixControlClient, UnixControlServer};

pub const MAX_CONTROL_FRAME_BYTES: usize = 65_536;
pub const MAX_REPLAY_ENTRIES: usize = 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlIdentity {
    pub platform_revision: u64,
    pub contract_digest: ContractDigest,
}

impl ControlIdentity {
    pub fn current() -> Result<Self, ControlError> {
        Ok(Self {
            platform_revision: PLATFORM_REVISION,
            contract_digest: control_contract()?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlError {
    Host(HostError),
    ContractUnavailable,
    InvalidIdentity,
    Malformed(&'static str),
    Oversized,
    Unauthorized(LocalPrincipal),
    ReplayConflict,
    Io(String),
}

impl fmt::Display for ControlError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => write!(output, "local control host failure: {error}"),
            Self::ContractUnavailable => output.write_str("runtime control contract unavailable"),
            Self::InvalidIdentity => output.write_str("invalid request or idempotency identity"),
            Self::Malformed(reason) => write!(output, "malformed local control frame: {reason}"),
            Self::Oversized => output.write_str("local control frame exceeds bound"),
            Self::Unauthorized(principal) => write!(
                output,
                "unauthorized local principal uid={} pid={}",
                principal.user, principal.process
            ),
            Self::ReplayConflict => {
                output.write_str("idempotency identity reused for different request")
            }
            Self::Io(message) => write!(output, "local control I/O: {message}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<HostError> for ControlError {
    fn from(value: HostError) -> Self {
        Self::Host(value)
    }
}

pub(crate) fn validate_request(request: &ControlRequest) -> Result<(), ControlFailure> {
    if request.identity.platform_revision != PLATFORM_REVISION {
        return Err(ControlFailure::StaleRevision {
            expected: PLATFORM_REVISION,
            found: request.identity.platform_revision,
        });
    }
    if request.identity.contract_digest
        != control_contract().map_err(|_| ControlFailure::Internal)?
    {
        return Err(ControlFailure::ContractMismatch);
    }
    if request.request_id == 0
        || (request.operation.modifies() && request.idempotency_id == [0; 32])
    {
        return Err(ControlFailure::Malformed);
    }
    Ok(())
}

fn control_contract() -> Result<ContractDigest, ControlError> {
    current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::RUNTIME_CONTROL)
                .map(RegisteredContract::digest)
        })
        .ok_or(ControlError::ContractUnavailable)
}
