use crate::{Error, Result};

use super::{
    CleanupFailure, CleanupFailureLimits, CleanupFailures, CleanupPhase, CleanupRetentionPolicy,
    CleanupSubject, ExecutionOutcome, HostError, ResourceLimitKind, Trap,
};

const MAX_NESTING: usize = 8;

include!("codec/limits.rs");

pub fn encode_execution_outcome(
    outcome: &ExecutionOutcome,
    limits: impl Into<ExecutionOutcomeCodecLimits>,
) -> Result<Vec<u8>> {
    let mut encoder = Encoder::new(limits.into().validate()?);
    encode_outcome(&mut encoder, outcome, 0)?;
    Ok(encoder.finish())
}

pub fn decode_execution_outcome(
    bytes: &[u8],
    limits: impl Into<ExecutionOutcomeCodecLimits>,
) -> Result<ExecutionOutcome> {
    let mut decoder = Decoder::new(bytes, limits.into().validate()?)?;
    let outcome = decode_outcome(&mut decoder, 0)?;
    decoder.finish()?;
    Ok(outcome)
}

fn encode_outcome(out: &mut Encoder, value: &ExecutionOutcome, depth: usize) -> Result<()> {
    if depth >= MAX_NESTING {
        return Err(Error::msg("execution outcome nesting exceeds bound"));
    }
    match value {
        ExecutionOutcome::Returned(value) => {
            out.u8(0)?;
            value.encode_wire(out)?;
        }
        ExecutionOutcome::Exited(code) => {
            out.u8(1)?;
            out.i32(*code)?;
        }
        ExecutionOutcome::Trapped(trap) => {
            out.u8(2)?;
            out.text(trap.as_str())?;
        }
        ExecutionOutcome::DeadlineExceeded => out.u8(3)?,
        ExecutionOutcome::ResourceLimitExceeded(kind) => {
            out.u8(4)?;
            out.u8(resource_tag(*kind))?;
        }
        ExecutionOutcome::HostFailure(error) => {
            out.u8(5)?;
            out.text(error.as_str())?;
        }
        ExecutionOutcome::CleanupFailed { primary, failures } => {
            out.u8(6)?;
            encode_outcome(out, primary, depth + 1)?;
            encode_cleanup(out, failures)?;
        }
    }
    Ok(())
}

fn decode_outcome(input: &mut Decoder<'_>, depth: usize) -> Result<ExecutionOutcome> {
    if depth >= MAX_NESTING {
        return Err(Error::msg("execution outcome nesting exceeds bound"));
    }
    Ok(match input.u8()? {
        0 => ExecutionOutcome::Returned(super::OwnedValue::decode_wire(input)?),
        1 => ExecutionOutcome::Exited(input.i32()?),
        2 => ExecutionOutcome::Trapped(Trap::new(input.text()?)),
        3 => ExecutionOutcome::DeadlineExceeded,
        4 => ExecutionOutcome::ResourceLimitExceeded(resource_kind(input.u8()?)?),
        5 => ExecutionOutcome::HostFailure(HostError::new(input.text()?)),
        6 => ExecutionOutcome::CleanupFailed {
            primary: Box::new(decode_outcome(input, depth + 1)?),
            failures: decode_cleanup(input)?,
        },
        _ => return Err(Error::msg("unknown execution outcome tag")),
    })
}

fn encode_cleanup(out: &mut Encoder, failures: &CleanupFailures) -> Result<()> {
    match failures.retention() {
        CleanupRetentionPolicy::Unrestricted => out.u8(0)?,
        CleanupRetentionPolicy::Limited(limits) => {
            out.u8(1)?;
            out.usize(limits.max_failures)?;
            out.usize(limits.max_message_bytes)?;
        }
    }
    out.usize(failures.retained().len())?;
    for failure in failures.retained() {
        out.u8(phase_tag(failure.phase()))?;
        encode_subject(out, failure.subject())?;
        out.text(failure.message())?;
        out.usize(failure.omitted_message_bytes())?;
    }
    out.usize(failures.retained_message_bytes())?;
    out.usize(failures.omitted_message_bytes())?;
    out.usize(failures.omitted_failures())
}

fn decode_cleanup(input: &mut Decoder<'_>) -> Result<CleanupFailures> {
    let retention = match input.u8()? {
        0 => CleanupRetentionPolicy::Unrestricted,
        1 => CleanupRetentionPolicy::Limited(
            CleanupFailureLimits::new(input.usize()?, input.usize()?)
                .ok_or_else(|| Error::msg("cleanup failure limits exceed bounds"))?,
        ),
        _ => return Err(Error::msg("unknown cleanup retention tag")),
    };
    let count = input.usize()?;
    if matches!(retention, CleanupRetentionPolicy::Limited(limits) if count > limits.max_failures) {
        return Err(Error::msg("cleanup failure count exceeds encoded limit"));
    }
    let mut retained = Vec::with_capacity(count);
    for _ in 0..count {
        retained.push(CleanupFailure::from_wire_parts(
            decode_phase(input.u8()?)?,
            decode_subject(input)?,
            input.text()?,
            input.usize()?,
        ));
    }
    CleanupFailures::from_wire_parts(
        retention,
        retained,
        input.usize()?,
        input.usize()?,
        input.usize()?,
    )
}

fn resource_tag(kind: ResourceLimitKind) -> u8 {
    match kind {
        ResourceLimitKind::InstructionFuel => 0,
        ResourceLimitKind::StackValues => 1,
        ResourceLimitKind::FrameDepth => 2,
        ResourceLimitKind::HeapBytes => 3,
        ResourceLimitKind::Allocations => 4,
        ResourceLimitKind::Handles => 6,
        ResourceLimitKind::OutputBytes => 7,
    }
}

fn resource_kind(tag: u8) -> Result<ResourceLimitKind> {
    Ok(match tag {
        0 => ResourceLimitKind::InstructionFuel,
        1 => ResourceLimitKind::StackValues,
        2 => ResourceLimitKind::FrameDepth,
        3 => ResourceLimitKind::HeapBytes,
        4 => ResourceLimitKind::Allocations,
        6 => ResourceLimitKind::Handles,
        7 => ResourceLimitKind::OutputBytes,
        _ => return Err(Error::msg("unknown resource limit tag")),
    })
}

include!("codec/cleanup_tags.rs");
include!("codec/io.rs");

#[cfg(test)]
mod structural_bounds_tests;
#[cfg(test)]
mod structural_decode_bounds_tests;
#[cfg(test)]
mod structural_tests;
#[cfg(test)]
mod tests;
