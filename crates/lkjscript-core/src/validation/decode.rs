use std::collections::HashSet;

use crate::{
    Chunk, DecodedInstruction, DecodedOperand, Error, FunctionProto, Op, OperandLayout, Result,
};

pub(super) fn decode_function(proto: &FunctionProto) -> Result<Vec<DecodedInstruction>> {
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
        let layout = op.operand_layout();
        let width = layout.byte_width();
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
        let operand = match layout {
            OperandLayout::None => DecodedOperand::None,
            OperandLayout::U16 => {
                let [low, high] = bytes else {
                    return Err(Error::msg(format!(
                        "function {} has invalid U16 operand layout for {op:?}",
                        proto.name
                    )));
                };
                DecodedOperand::U16(u16::from_le_bytes([*low, *high]))
            }
            OperandLayout::Index => {
                let value = decoded_u64(bytes, proto, op, instruction_offset)?;
                let value = usize::try_from(value).map_err(|_| {
                    Error::msg(format!(
                        "function {} {op:?} operand at byte {instruction_offset} exceeds host usize",
                        proto.name
                    ))
                })?;
                DecodedOperand::Index(value)
            }
            OperandLayout::PlaceLocal => {
                let (place, local) = bytes.split_at(8);
                let place = usize::try_from(decoded_u64(place, proto, op, instruction_offset)?)
                    .map_err(|_| {
                        Error::msg(format!(
                            "function {} {op:?} place at byte {instruction_offset} exceeds host usize",
                            proto.name
                        ))
                    })?;
                let local = usize::try_from(decoded_u64(local, proto, op, instruction_offset)?)
                    .map_err(|_| {
                        Error::msg(format!(
                            "function {} {op:?} local at byte {instruction_offset} exceeds host usize",
                            proto.name
                        ))
                    })?;
                DecodedOperand::PlaceLocal { place, local }
            }
        };
        offset = end;
        if instructions.len() == instructions.capacity() {
            instructions
                .try_reserve(1)
                .map_err(|_| Error::host("decoded instruction reservation failed"))?;
        }
        instructions.push(DecodedInstruction::new(
            instruction_offset,
            offset,
            op,
            operand,
        ));
    }
    Ok(instructions)
}

fn decoded_u64(
    bytes: &[u8],
    proto: &FunctionProto,
    op: Op,
    instruction_offset: usize,
) -> Result<u64> {
    let bytes: [u8; 8] = bytes.try_into().map_err(|_| {
        Error::msg(format!(
            "function {} has invalid U64 operand layout for {op:?} at byte {instruction_offset}",
            proto.name
        ))
    })?;
    Ok(u64::from_le_bytes(bytes))
}

include!("decode/operands.rs");

pub(super) fn instruction_error(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Error {
    Error::msg(format!(
        "function {} {op:?} at byte {at}: {message}",
        proto.name
    ))
}
