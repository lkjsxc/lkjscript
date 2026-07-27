use super::{instruction_error, types::*, Kind, State};
use crate::{DecodedInstruction, FunctionProto, ResourceKind, Result};

pub(super) fn file_open(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    kind: ResourceKind,
) -> Result<()> {
    expect_pop(state, Kind::Path, proto, instruction)?;
    expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
    state.stack.push(resource_result_kind(kind));
    Ok(())
}

pub(super) fn expect_resource(
    state: &mut State,
    allowed: &[ResourceKind],
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let actual = pop(state, proto, instruction)?;
    if matches!(actual, Kind::Resource(kind) if allowed.contains(&kind)) {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!("typed resource kind mismatch: got {actual}"),
        ))
    }
}

pub(super) fn expect_capability(
    state: &mut State,
    kind: crate::CapabilityKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::Capability(kind), proto, instruction)
}
