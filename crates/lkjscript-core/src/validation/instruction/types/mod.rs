use super::{instruction_error, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, ProductId, Result};

include!("structural.rs");

pub(super) fn instruction_operand(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<usize> {
    instruction.operand().index().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "decoded operand is missing",
        )
    })
}

pub(super) fn product_descriptor(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<crate::ProductFieldRef> {
    let index = instruction_operand(proto, instruction)?;
    chunk.product_fields.get(index).copied().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "product descriptor is missing",
        )
    })
}

pub(super) fn top(
    state: &State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    state.stack.last().copied().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operand stack underflow",
        )
    })
}

pub(super) fn pop(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    state.stack.pop().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operand stack underflow",
        )
    })
}

pub(super) fn expect_pop(
    state: &mut State,
    expected: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let actual = pop(state, proto, instruction)?;
    if actual == expected || actual == Kind::Any {
        return Ok(());
    }
    Err(instruction_error(
        proto,
        instruction.op(),
        instruction.offset(),
        &format!("operation category mismatch: expected {expected}, got {actual}"),
    ))
}

pub(super) fn expect_product(
    actual: Kind,
    expected: ProductId,
    region: bool,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let expected_kind = if region {
        Kind::RegionProduct(expected)
    } else {
        Kind::Product(expected)
    };
    if actual == Kind::Any || actual == expected_kind {
        return Ok(());
    }
    Err(instruction_error(
        proto,
        instruction.op(),
        instruction.offset(),
        &format!(
            "product operation category or identity mismatch: expected product {}, got {actual}",
            expected.raw()
        ),
    ))
}

pub(super) fn expect_numeric(
    actual: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if matches!(actual, Kind::Any | Kind::I64 | Kind::F64) {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!("numeric operation category mismatch: got {actual}"),
        ))
    }
}

pub(super) fn expect_two_numeric(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let right = pop(state, proto, instruction)?;
    let left = pop(state, proto, instruction)?;
    expect_numeric(left, proto, instruction)?;
    expect_numeric(right, proto, instruction)
}

pub(super) const fn result_kind() -> Kind {
    Kind::Enum(crate::EnumId::new(crate::RESULT_ID), None)
}

pub(super) const fn option_kind() -> Kind {
    Kind::Enum(crate::EnumId::new(crate::OPTION_ID), None)
}

include!("resources.rs");

pub(super) fn is_value_comparable(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Unit
            | Kind::Bool
            | Kind::I64
            | Kind::F64
            | Kind::Str
            | Kind::Symbol
            | Kind::Enum(_, _)
    )
}
