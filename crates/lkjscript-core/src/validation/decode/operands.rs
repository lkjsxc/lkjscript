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
            Op::LoadLocal
            | Op::StoreLocal
            | Op::ByteVectorBorrow
            | Op::ByteVectorBorrowMut
            | Op::BytesBorrow
            | Op::StoreUniqueLocal
            | Op::StoreViewLocal
            | Op::TakeUniqueLocal
            | Op::LoadViewLocal
            | Op::EndBorrowLocal => {
                let index = operand_index(operand, proto, op, at)?;
                if index >= usize::from(proto.locals) {
                    return operand_error(proto, op, at, "local index out of range");
                }
            }
            Op::ByteVectorPlaceInit
            | Op::ByteVectorMove
            | Op::ByteVectorDropPlace
            | Op::BytesPlaceInit
            | Op::BytesMove
            | Op::BytesDropPlace => {
                let packed = operand_index(operand, proto, op, at)?;
                let slot = packed & usize::from(u8::MAX);
                let place = packed >> u8::BITS;
                if slot >= usize::from(proto.locals) {
                    return operand_error(proto, op, at, "unique local index out of range");
                }
                if place >= usize::from(proto.unique_places) {
                    return operand_error(proto, op, at, "unique place index out of range");
                }
            }
            Op::ByteVectorPlaceEnd | Op::BytesPlaceEnd => {
                let place = operand_index(operand, proto, op, at)?;
                if place >= usize::from(proto.unique_places) {
                    return operand_error(proto, op, at, "unique place index out of range");
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
