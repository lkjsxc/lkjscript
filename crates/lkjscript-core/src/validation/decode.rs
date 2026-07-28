use std::collections::HashSet;

use crate::{
    Chunk, DecodedInstruction, Error, FunctionProto, Op, Result, ValidationLimits,
    MAX_FUNCTION_CODE_BYTES,
};

pub(super) fn decode_function(
    proto: &FunctionProto,
    limits: &ValidationLimits,
) -> Result<Vec<DecodedInstruction>> {
    let code_limit = limits.max_function_code_bytes.min(MAX_FUNCTION_CODE_BYTES);
    if proto.code.len() > code_limit {
        return Err(Error::msg(format!(
            "function {} has {} code bytes, limit {code_limit}",
            proto.name,
            proto.code.len()
        )));
    }
    if proto.code.is_empty() {
        return Err(Error::msg(format!(
            "function {} has no bytecode",
            proto.name
        )));
    }
    let mut instructions = Vec::new();
    let mut offset = 0_usize;
    while offset < proto.code.len() {
        let instruction_offset = offset;
        let byte = proto.code[offset];
        let op = Op::from_byte(byte).ok_or_else(|| {
            Error::msg(format!(
                "function {} has unknown or retired opcode {byte} at byte {instruction_offset}",
                proto.name
            ))
        })?;
        offset += 1;
        let width = op.operand_width();
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::msg(format!(
                "function {} operand offset overflow at byte {instruction_offset}",
                proto.name
            ))
        })?;
        let bytes = proto.code.get(offset..end).ok_or_else(|| {
            Error::msg(format!(
                "function {} has truncated {op:?} operand at byte {instruction_offset}",
                proto.name
            ))
        })?;
        let operand = match bytes {
            [] => None,
            [value] => Some(u16::from(*value)),
            [low, high] => Some(u16::from_le_bytes([*low, *high])),
            _ => {
                return Err(Error::msg(format!(
                    "function {} has unsupported operand width {width} for {op:?}",
                    proto.name
                )));
            }
        };
        offset = end;
        instructions.push(DecodedInstruction::new(
            instruction_offset,
            offset,
            op,
            operand,
        ));
    }
    Ok(instructions)
}

include!("decode/operands.rs");

pub(super) fn instruction_error(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Error {
    Error::msg(format!(
        "function {} {op:?} at byte {at}: {message}",
        proto.name
    ))
}
