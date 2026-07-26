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

pub(super) fn validate_instruction_operands(
    chunk: &Chunk,
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
) -> Result<()> {
    let boundaries: HashSet<usize> = instructions.iter().map(|item| item.offset()).collect();
    for instruction in instructions {
        let op = instruction.op();
        let operand = instruction.operand();
        let at = instruction.offset();
        match op {
            Op::LoadConst => {
                let index = operand_index(operand, proto, op, at)?;
                if index >= chunk.constants.len() {
                    return operand_error(proto, op, at, "constant index out of range");
                }
            }
            Op::LoadLocal | Op::StoreLocal => {
                let index = operand_index(operand, proto, op, at)?;
                if index >= usize::from(proto.locals) {
                    return operand_error(proto, op, at, "local index out of range");
                }
            }
            Op::LoadGlobal | Op::StoreGlobal => {
                let index = operand_index(operand, proto, op, at)?;
                if index >= chunk.global_names.len() {
                    return operand_error(proto, op, at, "global index out of range");
                }
            }
            Op::Jump | Op::JumpIfFalse => {
                let target = operand_index(operand, proto, op, at)?;
                if !boundaries.contains(&target) {
                    return operand_error(
                        proto,
                        op,
                        at,
                        "jump target is out of range or not an instruction boundary",
                    );
                }
            }
            Op::MakeClosure => {
                let captures = operand_index(operand, proto, op, at)?;
                if captures != 0 {
                    return operand_error(
                        proto,
                        op,
                        at,
                        "closure capture metadata is unsupported and must be zero",
                    );
                }
            }
            Op::MakeProduct => {
                let product = operand_index(operand, proto, op, at)?;
                if chunk.products.get(product).is_none() {
                    return operand_error(proto, op, at, "product index out of range");
                }
            }
            Op::LoadProductField | Op::WithProductField => {
                let descriptor = operand_index(operand, proto, op, at)?;
                if chunk.product_fields.get(descriptor).is_none() {
                    return operand_error(proto, op, at, "product descriptor index out of range");
                }
            }
            Op::MakeEnum => {
                let descriptor = operand_index(operand, proto, op, at)?;
                if chunk.enum_constructions.get(descriptor).is_none() {
                    return operand_error(
                        proto,
                        op,
                        at,
                        "enum construction descriptor out of range",
                    );
                }
            }
            Op::IsEnumVariant => {
                let descriptor = operand_index(operand, proto, op, at)?;
                if chunk.enum_variants.get(descriptor).is_none() {
                    return operand_error(proto, op, at, "enum variant descriptor out of range");
                }
            }
            Op::LoadEnumField => {
                let descriptor = operand_index(operand, proto, op, at)?;
                if chunk.enum_fields.get(descriptor).is_none() {
                    return operand_error(proto, op, at, "enum field descriptor out of range");
                }
            }
            Op::Call => {
                let _argc = operand_index(operand, proto, op, at)?;
            }
            _ => {
                if operand.is_some() {
                    return operand_error(proto, op, at, "unexpected encoded operand");
                }
            }
        }
    }
    Ok(())
}

fn operand_index(operand: Option<u16>, proto: &FunctionProto, op: Op, at: usize) -> Result<usize> {
    operand
        .map(usize::from)
        .ok_or_else(|| instruction_error(proto, op, at, "missing decoded operand"))
}

fn operand_error<T>(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Result<T> {
    Err(instruction_error(proto, op, at, message))
}

pub(super) fn instruction_error(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Error {
    Error::msg(format!(
        "function {} {op:?} at byte {at}: {message}",
        proto.name
    ))
}
