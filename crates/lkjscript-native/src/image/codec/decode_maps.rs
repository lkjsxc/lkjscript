use super::{reader::Reader, values, ImageCodecError};
use crate::image::{outcome_map_entry, source_map_entry, trap_map_entry};
use crate::*;

pub(super) fn source(input: &mut Reader<'_>, plan: u64) -> Result<SourceMapEntry, ImageCodecError> {
    Ok(source_map_entry(
        values::read_function(input, plan)?,
        input.u32()?,
        input.u32()?,
        values::read_source(input)?,
    ))
}

pub(super) fn trap(input: &mut Reader<'_>, plan: u64) -> Result<TrapMapEntry, ImageCodecError> {
    let function = values::read_function(input, plan)?;
    let offset = input.u32()?;
    let trap = trap_code(input.u32()?)?;
    let site = match input.u8()? {
        0 => None,
        1 => Some(input.u32()?),
        _ => return Err(ImageCodecError::new("noncanonical trap-site option")),
    };
    Ok(trap_map_entry(function, offset, trap, site))
}

pub(super) fn outcome(
    input: &mut Reader<'_>,
    plan: u64,
) -> Result<OutcomeMapEntry, ImageCodecError> {
    let function = values::read_function(input, plan)?;
    let offset = input.u32()?;
    let outcome = match input.u8()? {
        0 => OutcomeKind::Return,
        1 => OutcomeKind::Trap(trap_code(input.u32()?)?),
        2 => OutcomeKind::Exit,
        3 => OutcomeKind::DeadlineExceeded,
        4 => OutcomeKind::ResourceLimitExceeded,
        5 => OutcomeKind::HostFailure,
        _ => return Err(ImageCodecError::new("unknown outcome kind")),
    };
    Ok(outcome_map_entry(function, offset, outcome))
}

fn trap_code(value: u32) -> Result<TrapCode, ImageCodecError> {
    match value {
        1 => Ok(TrapCode::I64Overflow),
        2 => Ok(TrapCode::DivisionByZero),
        3 => Ok(TrapCode::Explicit),
        _ => Err(ImageCodecError::new("unknown trap code")),
    }
}
