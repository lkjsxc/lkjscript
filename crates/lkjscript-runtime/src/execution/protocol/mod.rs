use std::io::{self, Read, Write};

use lkjscript_contracts::PreparedProgramIdentity;
use lkjscript_core::{
    CapabilityKind, ExecutionOutcome, ExecutionOutcomeCodecLimits, ExecutionPolicy,
    LimitedExecutionPolicy,
};

pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ENTRY_BYTES: usize = 4 * 1024;
pub const MAX_ARGUMENTS: usize = 256;
pub const MAX_ARGUMENT_BYTES: usize = 4 * 1024;
pub const MAX_AGGREGATE_ARGUMENT_BYTES: usize = 256 * 1024;
pub const MAX_APPLICATION_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_DIAGNOSTIC_BYTES: usize = 4 * 1024;
pub const MAX_FLUSHES: u64 = 1_000_000;
pub const PROCESS_OUTCOME_CODEC_LIMITS: ExecutionOutcomeCodecLimits =
    ExecutionOutcomeCodecLimits::new(MAX_FRAME_BYTES);

pub fn runtime_control_digest() -> io::Result<[u8; 32]> {
    let contracts = lkjscript_contracts::current_contracts()
        .map_err(|error| invalid(format!("runtime contracts: {error}")))?;
    contracts
        .get(lkjscript_contracts::RUNTIME_CONTROL)
        .map(|contract| contract.digest().as_bytes())
        .ok_or_else(|| invalid("runtime-control contract is not registered"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessBootstrap {
    pub platform_revision: u64,
    pub contract: [u8; 32],
    pub coordinator: u64,
    pub application: u64,
    pub incarnation: u64,
    pub package: [u8; 32],
    pub entry: String,
    pub expected_entry: [u8; 32],
    pub expected_prepared: PreparedProgramIdentity,
    pub expected_return_semantic: [u8; 32],
    pub expected_root_witness_group: [u8; 32],
    pub expected_root_witness_member: [u8; 32],
    pub capabilities: Vec<CapabilityKind>,
    pub execution: ExecutionPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessRequest {
    Invoke { cell: u64, arguments: Vec<String> },
    Stop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessProgramProvenance {
    pub platform_revision: u64,
    pub contract: [u8; 32],
    pub application: u64,
    pub incarnation: u64,
    pub package: [u8; 32],
    pub entry: [u8; 32],
    pub prepared: PreparedProgramIdentity,
    pub return_semantic: [u8; 32],
    pub root_witness_group: [u8; 32],
    pub root_witness_member: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessResponse {
    Ready {
        process: u32,
        provenance: ProcessProgramProvenance,
    },
    ReadyFailure {
        diagnostic: String,
    },
    Outcome {
        provenance: ProcessProgramProvenance,
        cell: u64,
        outcome: ExecutionOutcome,
        output: Vec<u8>,
        flushes: u64,
    },
    Stopped,
}

pub fn expected_process_provenance(value: &ProcessBootstrap) -> ProcessProgramProvenance {
    ProcessProgramProvenance {
        platform_revision: value.platform_revision,
        contract: value.contract,
        application: value.application,
        incarnation: value.incarnation,
        package: value.package,
        entry: value.expected_entry,
        prepared: value.expected_prepared,
        return_semantic: value.expected_return_semantic,
        root_witness_group: value.expected_root_witness_group,
        root_witness_member: value.expected_root_witness_member,
    }
}

pub fn validate_process_provenance(
    expected: &ProcessProgramProvenance,
    actual: &ProcessProgramProvenance,
) -> io::Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(invalid("process outcome provenance mismatch"))
    }
}

pub fn write_bootstrap(output: &mut impl Write, value: &ProcessBootstrap) -> io::Result<()> {
    let mut body = Writer::new();
    body.u8(0)?;
    encode_bootstrap(&mut body, value)?;
    write_frame(output, body.finish())
}

pub fn read_bootstrap(input: &mut impl Read) -> io::Result<ProcessBootstrap> {
    let frame = read_frame(input)?;
    let mut body = Reader::new(&frame);
    if body.u8()? != 0 {
        return Err(invalid("expected process bootstrap"));
    }
    let value = decode_bootstrap(&mut body)?;
    body.finish()?;
    Ok(value)
}

pub fn write_request(output: &mut impl Write, value: &ProcessRequest) -> io::Result<()> {
    let mut body = Writer::new();
    match value {
        ProcessRequest::Invoke { cell, arguments } => {
            body.u8(1)?;
            body.u64(*cell)?;
            encode_arguments(&mut body, arguments)?;
        }
        ProcessRequest::Stop => body.u8(2)?,
    }
    write_frame(output, body.finish())
}

pub fn read_request(input: &mut impl Read) -> io::Result<ProcessRequest> {
    let frame = read_frame(input)?;
    let mut body = Reader::new(&frame);
    let value = match body.u8()? {
        1 => ProcessRequest::Invoke {
            cell: nonzero(body.u64()?, "execution cell")?,
            arguments: decode_arguments(&mut body)?,
        },
        2 => ProcessRequest::Stop,
        _ => return Err(invalid("unknown process request")),
    };
    body.finish()?;
    Ok(value)
}

pub fn write_response(output: &mut impl Write, value: &ProcessResponse) -> io::Result<()> {
    let mut body = Writer::new();
    encode_response(&mut body, value)?;
    write_frame(output, body.finish())
}

pub fn read_response(input: &mut impl Read) -> io::Result<ProcessResponse> {
    let frame = read_frame(input)?;
    let mut body = Reader::new(&frame);
    let value = decode_response(&mut body)?;
    body.finish()?;
    Ok(value)
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn nonzero(value: u64, name: &str) -> io::Result<u64> {
    if value == 0 {
        Err(invalid(format!("{name} must be nonzero")))
    } else {
        Ok(value)
    }
}

include!("io.rs");
include!("messages.rs");
include!("config.rs");

#[cfg(test)]
mod bootstrap_tests;
#[cfg(test)]
mod semantic_dag_tests;
#[cfg(test)]
mod structural_tests;
#[cfg(test)]
mod tests;
