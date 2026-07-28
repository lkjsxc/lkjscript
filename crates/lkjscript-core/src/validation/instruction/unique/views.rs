use super::support::*;
use super::{Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::StoreViewLocal => store_view(proto, instruction, state)?,
        Op::LoadViewLocal => load_view(proto, instruction, state)?,
        Op::ByteSliceLen => {
            pop_used_view(state, false, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::ByteSliceRef => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_used_view(state, false, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::ByteSliceMutSet => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_used_view(state, true, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::EndBorrowLocal => end_borrow(proto, instruction, state)?,
        _ => unreachable!("view opcode family checked"),
    }
    Ok(())
}

fn store_view(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = pop(state, proto, instruction)?;
    if !matches!(value, Kind::ByteSlice { used: false, .. }) {
        return Err(error(
            proto,
            instruction,
            "StoreViewLocal expects a fresh byte view",
        ));
    }
    store_empty_local(state, slot, value, proto, instruction)
}

fn load_view(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let kind = state
        .locals
        .get(slot)
        .copied()
        .flatten()
        .ok_or_else(|| error(proto, instruction, "byte view local is not initialized"))?;
    let Kind::ByteSlice {
        owner,
        mutable,
        used: false,
    } = kind
    else {
        return Err(error(
            proto,
            instruction,
            "byte view local is stale, already used, or has the wrong type",
        ));
    };
    let used = Kind::ByteSlice {
        owner,
        mutable,
        used: true,
    };
    state.locals[slot] = Some(used);
    state.stack.push(used);
    Ok(())
}

fn end_borrow(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state
        .locals
        .get(slot)
        .copied()
        .flatten()
        .ok_or_else(|| error(proto, instruction, "EndBorrow local is not initialized"))?;
    let Kind::ByteSlice {
        owner, used: true, ..
    } = value
    else {
        return Err(error(
            proto,
            instruction,
            "EndBorrow expects one used exact byte view",
        ));
    };
    if owner & 0xf000_0000 == 0x9000_0000 {
        return Err(error(
            proto,
            instruction,
            "borrowed parameter view cannot be ended by its callee",
        ));
    }
    if state
        .stack
        .iter()
        .any(|kind| matches!(kind, Kind::ByteSlice { owner: active, .. } if *active == owner))
    {
        return Err(error(
            proto,
            instruction,
            "EndBorrow has a live operand-stack view",
        ));
    }
    state.locals[slot] = None;
    state.stack.push(Kind::Unit);
    Ok(())
}

fn pop_used_view(
    state: &mut State,
    mutable: bool,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<u32> {
    match pop(state, proto, instruction)? {
        Kind::ByteSlice {
            owner,
            mutable: actual,
            used: true,
        } if actual == mutable => Ok(owner),
        actual => Err(error(
            proto,
            instruction,
            &format!("byte view operation has wrong or unused view type: {actual}"),
        )),
    }
}
