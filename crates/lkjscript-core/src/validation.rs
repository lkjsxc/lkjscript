//! Single whole-chunk bytecode validation boundary.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    Chunk, Constant, DecodedInstruction, Error, FunctionProto, Op, ProductId, Result, StackEffect,
    ValidationLimits, MAX_FUNCTION_CODE_BYTES, MAX_PRODUCT_FIELDS,
};

#[derive(Debug, Clone)]
pub struct ValidatedChunk {
    chunk: Chunk,
    main_instructions: Vec<DecodedInstruction>,
    proto_instructions: Vec<Vec<DecodedInstruction>>,
}

impl ValidatedChunk {
    pub fn constants(&self) -> &[Constant] {
        &self.chunk.constants
    }

    pub fn protos(&self) -> &[FunctionProto] {
        &self.chunk.protos
    }

    pub fn main(&self) -> &FunctionProto {
        &self.chunk.main
    }

    pub fn global_names(&self) -> &[String] {
        &self.chunk.global_names
    }

    pub fn products(&self) -> &[crate::ProductMetadata] {
        &self.chunk.products
    }

    pub fn product_fields(&self) -> &[crate::ProductFieldRef] {
        &self.chunk.product_fields
    }

    pub fn main_instructions(&self) -> &[DecodedInstruction] {
        &self.main_instructions
    }

    pub fn proto_instructions(&self, index: usize) -> Option<&[DecodedInstruction]> {
        self.proto_instructions.get(index).map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Any,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    Proto(u32),
    Closure(u32),
    List,
    Buf,
    Handle,
    Result,
    Option,
    Product(ProductId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Kind>,
    locals: Vec<Option<Kind>>,
    globals: Vec<Option<Kind>>,
}

pub fn validate_chunk(chunk: Chunk, limits: &ValidationLimits) -> Result<ValidatedChunk> {
    validate_tables(&chunk, limits)?;
    let main_instructions = decode_function(&chunk.main, limits)?;
    let mut proto_instructions = Vec::with_capacity(chunk.protos.len());
    for proto in &chunk.protos {
        proto_instructions.push(decode_function(proto, limits)?);
    }

    validate_instruction_operands(&chunk, &chunk.main, &main_instructions)?;
    validate_control_flow(&chunk, &chunk.main, &main_instructions, true)?;
    for (index, proto) in chunk.protos.iter().enumerate() {
        let instructions = proto_instructions
            .get(index)
            .ok_or_else(|| Error::msg("validator prototype decode metadata is inconsistent"))?;
        validate_instruction_operands(&chunk, proto, instructions)?;
        validate_control_flow(&chunk, proto, instructions, false)?;
    }

    Ok(ValidatedChunk {
        chunk,
        main_instructions,
        proto_instructions,
    })
}

fn validate_tables(chunk: &Chunk, limits: &ValidationLimits) -> Result<()> {
    let tables = [
        ("constants", chunk.constants.len()),
        ("prototypes", chunk.protos.len()),
        ("globals", chunk.global_names.len()),
        ("products", chunk.products.len()),
        ("product field descriptors", chunk.product_fields.len()),
    ];
    for (name, length) in tables {
        if length > limits.max_table_entries {
            return Err(Error::msg(format!(
                "bytecode {name} table has {length} entries, limit {}",
                limits.max_table_entries
            )));
        }
    }
    if chunk.main.arity != 0 {
        return Err(Error::msg("bytecode main entry must have arity 0"));
    }
    validate_proto_shape(&chunk.main, "main")?;

    let mut function_names = HashSet::with_capacity(chunk.protos.len());
    for proto in &chunk.protos {
        validate_proto_shape(proto, "prototype")?;
        if proto.name.is_empty() {
            return Err(Error::msg("bytecode prototype has an empty name"));
        }
        if !function_names.insert(proto.name.as_str()) {
            return Err(Error::msg(format!(
                "duplicate bytecode prototype name {}",
                proto.name
            )));
        }
    }

    let mut metadata_bytes = chunk.main.name.len();
    let mut encoded_bytes = chunk.main.code.len();
    for proto in &chunk.protos {
        metadata_bytes = checked_add(metadata_bytes, proto.name.len(), "metadata byte size")?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }

    let mut global_names = HashSet::with_capacity(chunk.global_names.len());
    for name in &chunk.global_names {
        if name.is_empty() {
            return Err(Error::msg("bytecode global has an empty name"));
        }
        if !global_names.insert(name.as_str()) {
            return Err(Error::msg(format!("duplicate bytecode global name {name}")));
        }
        metadata_bytes = checked_add(metadata_bytes, name.len(), "metadata byte size")?;
    }

    let mut product_names = HashSet::with_capacity(chunk.products.len());
    for (index, product) in chunk.products.iter().enumerate() {
        if product.id.index() != index {
            return Err(Error::msg(format!(
                "product metadata index {index} has inconsistent ProductId {}",
                product.id.raw()
            )));
        }
        if product.name.is_empty() {
            return Err(Error::msg(format!(
                "product metadata {index} has an empty name"
            )));
        }
        if !product_names.insert(product.name.as_str()) {
            return Err(Error::msg(format!(
                "duplicate product metadata name {}",
                product.name
            )));
        }
        if product.fields.len() > MAX_PRODUCT_FIELDS {
            return Err(Error::msg(format!(
                "product metadata {} exceeds field limit {MAX_PRODUCT_FIELDS}",
                product.name
            )));
        }
        metadata_bytes = checked_add(metadata_bytes, product.name.len(), "metadata byte size")?;
        let mut fields = HashSet::with_capacity(product.fields.len());
        for field in &product.fields {
            if field.is_empty() {
                return Err(Error::msg(format!(
                    "product metadata {} has an empty field name",
                    product.name
                )));
            }
            if !fields.insert(field.as_str()) {
                return Err(Error::msg(format!(
                    "product metadata {} has duplicate field {field}",
                    product.name
                )));
            }
            metadata_bytes = checked_add(metadata_bytes, field.len(), "metadata byte size")?;
        }
    }

    let mut descriptors = HashSet::with_capacity(chunk.product_fields.len());
    for (index, field_ref) in chunk.product_fields.iter().copied().enumerate() {
        let product = chunk
            .products
            .get(field_ref.product.index())
            .ok_or_else(|| {
                Error::msg(format!(
                    "product field descriptor {index} has an unknown ProductId {}",
                    field_ref.product.raw()
                ))
            })?;
        if product.id != field_ref.product {
            return Err(Error::msg(format!(
                "product field descriptor {index} has inconsistent ProductId {}",
                field_ref.product.raw()
            )));
        }
        if usize::from(field_ref.field) >= product.fields.len() {
            return Err(Error::msg(format!(
                "product field descriptor {index} field {} is out of range",
                field_ref.field
            )));
        }
        if !descriptors.insert(field_ref) {
            return Err(Error::msg(format!(
                "duplicate product field descriptor at index {index}"
            )));
        }
    }

    for (index, constant) in chunk.constants.iter().enumerate() {
        encoded_bytes = checked_add(encoded_bytes, 1, "encoded byte size")?;
        match constant {
            Constant::I64(_) | Constant::F64(_) => {
                encoded_bytes = checked_add(encoded_bytes, 8, "encoded byte size")?;
            }
            Constant::Str(text) | Constant::Symbol(text) => {
                if text.len() > limits.max_constant_data_bytes {
                    return Err(Error::msg(format!(
                        "constant {index} has {} data bytes, limit {}",
                        text.len(),
                        limits.max_constant_data_bytes
                    )));
                }
                encoded_bytes = checked_add(encoded_bytes, text.len(), "encoded byte size")?;
            }
            Constant::Proto(proto) => {
                if usize::try_from(*proto)
                    .ok()
                    .is_none_or(|proto| proto >= chunk.protos.len())
                {
                    return Err(Error::msg(format!(
                        "constant {index} references prototype {proto} out of range"
                    )));
                }
                encoded_bytes = checked_add(encoded_bytes, 4, "encoded byte size")?;
            }
        }
    }

    if metadata_bytes > limits.max_metadata_bytes {
        return Err(Error::msg(format!(
            "bytecode metadata has {metadata_bytes} bytes, limit {}",
            limits.max_metadata_bytes
        )));
    }
    if encoded_bytes > limits.max_encoded_bytes {
        return Err(Error::msg(format!(
            "encoded bytecode has {encoded_bytes} bytes, limit {}",
            limits.max_encoded_bytes
        )));
    }
    Ok(())
}

fn validate_proto_shape(proto: &FunctionProto, category: &str) -> Result<()> {
    if proto.arity > proto.locals {
        return Err(Error::msg(format!(
            "bytecode {category} {} has arity {} greater than local count {}",
            proto.name, proto.arity, proto.locals
        )));
    }
    Ok(())
}

fn checked_add(left: usize, right: usize, category: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg(format!("bytecode {category} overflow")))
}

fn decode_function(
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

fn validate_instruction_operands(
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

fn instruction_error(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Error {
    Error::msg(format!(
        "function {} {op:?} at byte {at}: {message}",
        proto.name
    ))
}

fn validate_control_flow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    is_main: bool,
) -> Result<()> {
    let by_offset: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.offset(), index))
        .collect();
    let mut states = vec![None; instructions.len()];
    let mut locals = vec![None; usize::from(proto.locals)];
    for slot in locals.iter_mut().take(usize::from(proto.arity)) {
        *slot = Some(Kind::Any);
    }
    let globals = if is_main {
        vec![None; chunk.global_names.len()]
    } else {
        vec![Some(Kind::Any); chunk.global_names.len()]
    };
    states[0] = Some(State {
        stack: Vec::new(),
        locals,
        globals,
    });
    let mut pending = VecDeque::from([0_usize]);

    while let Some(index) = pending.pop_front() {
        let instruction = instructions
            .get(index)
            .copied()
            .ok_or_else(|| Error::msg("validator CFG instruction index out of range"))?;
        let mut state = states
            .get(index)
            .and_then(Clone::clone)
            .ok_or_else(|| Error::msg("validator CFG state is missing"))?;
        apply_instruction(chunk, proto, instruction, &mut state)?;

        let successors = successors(proto, instructions, &by_offset, index, instruction)?;
        for successor in successors {
            let target = states
                .get_mut(successor)
                .ok_or_else(|| Error::msg("validator CFG successor index out of range"))?;
            let changed = merge_state(target, &state, proto, instruction)?;
            if changed {
                pending.push_back(successor);
            }
        }
    }
    Ok(())
}

fn successors(
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    by_offset: &HashMap<usize, usize>,
    index: usize,
    instruction: DecodedInstruction,
) -> Result<Vec<usize>> {
    let target = || -> Result<usize> {
        let offset = instruction.operand().map(usize::from).ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "missing jump target",
            )
        })?;
        by_offset.get(&offset).copied().ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "jump target is not an instruction boundary",
            )
        })
    };
    match instruction.op().info().control {
        crate::ControlFlow::Return | crate::ControlFlow::Exit => Ok(Vec::new()),
        crate::ControlFlow::Jump => Ok(vec![target()?]),
        crate::ControlFlow::Branch => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable branch falls through the end of the function",
                    )
                })?;
            Ok(vec![target()?, next])
        }
        crate::ControlFlow::Next => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable execution falls through the end of the function",
                    )
                })?;
            Ok(vec![next])
        }
    }
}

fn merge_state(
    target: &mut Option<State>,
    incoming: &State,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
) -> Result<bool> {
    let Some(current) = target else {
        *target = Some(incoming.clone());
        return Ok(true);
    };
    if current.stack.len() != incoming.stack.len() {
        return Err(instruction_error(
            proto,
            predecessor.op(),
            predecessor.offset(),
            "incompatible operand stack depth at CFG join",
        ));
    }
    let mut changed = false;
    for (existing, incoming) in current.stack.iter_mut().zip(&incoming.stack) {
        let merged = merge_kind(*existing, *incoming).ok_or_else(|| {
            instruction_error(
                proto,
                predecessor.op(),
                predecessor.offset(),
                "incompatible operand stack categories at CFG join",
            )
        })?;
        if *existing != merged {
            *existing = merged;
            changed = true;
        }
    }
    for (existing, incoming) in current.locals.iter_mut().zip(&incoming.locals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "local")?;
    }
    for (existing, incoming) in current.globals.iter_mut().zip(&incoming.globals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "global")?;
    }
    Ok(changed)
}

fn merge_slot(
    existing: &mut Option<Kind>,
    incoming: Option<Kind>,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
    category: &str,
) -> Result<bool> {
    match (*existing, incoming) {
        (None, _) => Ok(false),
        (Some(_), None) => {
            *existing = None;
            Ok(true)
        }
        (Some(left), Some(right)) => {
            let merged = merge_kind(left, right).ok_or_else(|| {
                instruction_error(
                    proto,
                    predecessor.op(),
                    predecessor.offset(),
                    &format!("incompatible {category} categories at CFG join"),
                )
            })?;
            if Some(merged) == *existing {
                Ok(false)
            } else {
                *existing = Some(merged);
                Ok(true)
            }
        }
    }
}

fn merge_kind(left: Kind, right: Kind) -> Option<Kind> {
    if left == right {
        Some(left)
    } else if left == Kind::Any || right == Kind::Any {
        Some(Kind::Any)
    } else {
        None
    }
}

fn apply_instruction(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    let required = match op.info().stack {
        StackEffect::Fixed { required, .. } => required,
        StackEffect::Call => instruction
            .operand()
            .map(usize::from)
            .and_then(|argc| argc.checked_add(1))
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "call stack requirement overflow",
                )
            })?,
        StackEffect::MakeProduct => {
            let product = instruction
                .operand()
                .map(usize::from)
                .and_then(|index| chunk.products.get(index))
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "product metadata is missing",
                    )
                })?;
            product.fields.len()
        }
    };
    if state.stack.len() < required {
        return Err(instruction_error(
            proto,
            op,
            instruction.offset(),
            "operand stack underflow",
        ));
    }

    match op {
        Op::Nop | Op::Jump => {}
        Op::LoadConst => {
            let constant = instruction
                .operand()
                .map(usize::from)
                .and_then(|index| chunk.constants.get(index))
                .ok_or_else(|| {
                    instruction_error(proto, op, instruction.offset(), "constant is missing")
                })?;
            state.stack.push(match constant {
                Constant::I64(_) => Kind::I64,
                Constant::F64(_) => Kind::F64,
                Constant::Str(_) => Kind::Str,
                Constant::Symbol(_) => Kind::Symbol,
                Constant::Proto(proto) => Kind::Proto(*proto),
            });
        }
        Op::LoadLocal => {
            let slot = instruction_operand(proto, instruction)?;
            let kind = state.locals.get(slot).copied().flatten().ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "local is not definitely initialized",
                )
            })?;
            state.stack.push(kind);
        }
        Op::StoreLocal => {
            let slot = instruction_operand(proto, instruction)?;
            let value = top(state, proto, instruction)?;
            let target = state.locals.get_mut(slot).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "local index is out of range",
                )
            })?;
            *target = Some(value);
        }
        Op::LoadGlobal => {
            let slot = instruction_operand(proto, instruction)?;
            let kind = state.globals.get(slot).copied().flatten().ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "global is not definitely initialized",
                )
            })?;
            state.stack.push(kind);
        }
        Op::StoreGlobal => {
            let slot = instruction_operand(proto, instruction)?;
            let value = top(state, proto, instruction)?;
            let target = state.globals.get_mut(slot).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "global index is out of range",
                )
            })?;
            *target = Some(value);
        }
        Op::Add | Op::Sub | Op::Mul | Op::Div => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            expect_numeric(left, proto, instruction)?;
            expect_numeric(right, proto, instruction)?;
            state
                .stack
                .push(if left == Kind::Any || right == Kind::Any {
                    Kind::Any
                } else if left == Kind::F64 || right == Kind::F64 {
                    Kind::F64
                } else {
                    Kind::I64
                });
        }
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            expect_two_numeric(state, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::BitAnd | Op::BitOr | Op::BitXor => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::EqualValue => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            if left != Kind::Any
                && right != Kind::Any
                && (left != right || !is_value_comparable(left))
            {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "incompatible equal-value categories",
                ));
            }
            state.stack.push(Kind::Bool);
        }
        Op::Not | Op::JumpIfFalse => {
            expect_pop(state, Kind::Bool, proto, instruction)?;
            if op == Op::Not {
                state.stack.push(Kind::Bool);
            }
        }
        Op::Call => {
            let argc = instruction_operand(proto, instruction)?;
            let callee = pop(state, proto, instruction)?;
            let callee_proto = match callee {
                Kind::Closure(index) => Some(index),
                Kind::Any => None,
                _ => {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "call expects Closure",
                    ));
                }
            };
            for _ in 0..argc {
                let _argument = pop(state, proto, instruction)?;
            }
            if let Some(callee_proto) = callee_proto {
                let callee_proto = usize::try_from(callee_proto)
                    .ok()
                    .and_then(|index| chunk.protos.get(index))
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            op,
                            instruction.offset(),
                            "closure prototype index is out of range",
                        )
                    })?;
                if usize::from(callee_proto.arity) != argc {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "statically known call arity mismatch",
                    ));
                }
            }
            state.stack.push(Kind::Any);
        }
        Op::Return => {
            if state.stack.len() != 1 {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "return requires exactly one operand value",
                ));
            }
            let _returned = pop(state, proto, instruction)?;
        }
        Op::MakeClosure => {
            let value = pop(state, proto, instruction)?;
            let Kind::Proto(index) = value else {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "MakeClosure expects a prototype constant",
                ));
            };
            if usize::try_from(index)
                .ok()
                .is_none_or(|index| index >= chunk.protos.len())
            {
                return Err(instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "MakeClosure prototype is out of range",
                ));
            }
            state.stack.push(Kind::Closure(index));
        }
        Op::Cons => {
            expect_pop(state, Kind::List, proto, instruction)?;
            let _car = pop(state, proto, instruction)?;
            state.stack.push(Kind::List);
        }
        Op::Car => {
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
        Op::Cdr => {
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::List);
        }
        Op::IsEmptyList => {
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::SameObject => {
            let right = pop(state, proto, instruction)?;
            let left = pop(state, proto, instruction)?;
            if left != Kind::Any && right != Kind::Any {
                let valid = left == right && matches!(left, Kind::Buf | Kind::Handle);
                if !valid {
                    return Err(instruction_error(
                        proto,
                        op,
                        instruction.offset(),
                        "same-object expects matching Buf or Handle categories",
                    ));
                }
            }
            state.stack.push(Kind::Bool);
        }
        Op::ListEqual => {
            expect_pop(state, Kind::List, proto, instruction)?;
            expect_pop(state, Kind::List, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::F64BitsEqual => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            expect_pop(state, Kind::F64, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::Print => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Flush => state.stack.push(Kind::Unit),
        Op::ReadByte => state.stack.push(Kind::I64),
        Op::WriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Exit => {
            expect_pop(state, Kind::I64, proto, instruction)?;
        }
        Op::WriteStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::BufNew => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Buf);
        }
        Op::BufFromStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Buf);
        }
        Op::BufToStr => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::BufLen | Op::BufClone => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(if op == Op::BufLen {
                Kind::I64
            } else {
                Kind::Buf
            });
        }
        Op::BufRef | Op::BufGetU32 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::BufSet | Op::BufSetU32 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Pop => {
            let _value = pop(state, proto, instruction)?;
        }
        Op::Dup => {
            let value = top(state, proto, instruction)?;
            state.stack.push(value);
        }
        Op::SysTtyGet | Op::SysTtySet => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysPoll => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::StdinHandle => state.stack.push(Kind::Handle),
        Op::SysIsatty | Op::SysClose | Op::SysReadByte | Op::SysAccept | Op::SysRecv => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysTtyGuardSave => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysTtyGuardClear | Op::SysNowMs | Op::SysSocket => state.stack.push(Kind::Result),
        Op::False | Op::True => state.stack.push(Kind::Bool),
        Op::Unit => state.stack.push(Kind::Unit),
        Op::EmptyList => state.stack.push(Kind::List),
        Op::OptionNone => state.stack.push(Kind::Option),
        Op::StrLen => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::StrRef => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::StrAppend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrSlice => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrFromByte | Op::StrFromI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrFromF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenAppend
        | Op::SysOpenCreateNew
        | Op::SysOpenDir
        | Op::SysPathExists => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysFsync => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysWriteByte | Op::SysBind | Op::SysListen | Op::SysTruncate => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::Arg => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Option);
        }
        Op::Argc => state.stack.push(Kind::I64),
        Op::EmptyStr => state.stack.push(Kind::Str),
        Op::SysWaitMs => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysSend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysReadInto | Op::SysWriteFrom => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysRandomFill => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysRename => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::OkWrap | Op::ErrWrap => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::IsOk => {
            expect_pop(state, Kind::Result, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::UnwrapOk | Op::UnwrapErr => {
            expect_pop(state, Kind::Result, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
        Op::SomeWrap => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Option);
        }
        Op::IsSome => {
            expect_pop(state, Kind::Option, proto, instruction)?;
            state.stack.push(Kind::Bool);
        }
        Op::UnwrapSome => {
            expect_pop(state, Kind::Option, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
        Op::MakeProduct => {
            let product_index = instruction_operand(proto, instruction)?;
            let product = chunk.products.get(product_index).ok_or_else(|| {
                instruction_error(
                    proto,
                    op,
                    instruction.offset(),
                    "product metadata is missing",
                )
            })?;
            for _ in 0..product.fields.len() {
                let _field = pop(state, proto, instruction)?;
            }
            state.stack.push(Kind::Product(product.id));
        }
        Op::LoadProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            expect_product(product, descriptor.product, proto, instruction)?;
            state.stack.push(Kind::Any);
        }
        Op::WithProductField => {
            let descriptor = product_descriptor(chunk, proto, instruction)?;
            let _replacement = pop(state, proto, instruction)?;
            let product = pop(state, proto, instruction)?;
            expect_product(product, descriptor.product, proto, instruction)?;
            state.stack.push(Kind::Product(descriptor.product));
        }
    }
    Ok(())
}

fn instruction_operand(proto: &FunctionProto, instruction: DecodedInstruction) -> Result<usize> {
    instruction.operand().map(usize::from).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "decoded operand is missing",
        )
    })
}

fn product_descriptor(
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

fn top(state: &State, proto: &FunctionProto, instruction: DecodedInstruction) -> Result<Kind> {
    state.stack.last().copied().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operand stack underflow",
        )
    })
}

fn pop(state: &mut State, proto: &FunctionProto, instruction: DecodedInstruction) -> Result<Kind> {
    state.stack.pop().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operand stack underflow",
        )
    })
}

fn expect_pop(
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
        &format!("operation category mismatch: expected {expected:?}, got {actual:?}"),
    ))
}

fn expect_product(
    actual: Kind,
    expected: ProductId,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if actual == Kind::Any || actual == Kind::Product(expected) {
        return Ok(());
    }
    Err(instruction_error(
        proto,
        instruction.op(),
        instruction.offset(),
        &format!(
            "product operation category or identity mismatch: expected ProductId {}, got {actual:?}",
            expected.raw()
        ),
    ))
}

fn expect_numeric(
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
            &format!("numeric operation category mismatch: got {actual:?}"),
        ))
    }
}

fn expect_two_numeric(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let right = pop(state, proto, instruction)?;
    let left = pop(state, proto, instruction)?;
    expect_numeric(left, proto, instruction)?;
    expect_numeric(right, proto, instruction)
}

fn is_value_comparable(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Unit
            | Kind::Bool
            | Kind::I64
            | Kind::F64
            | Kind::Str
            | Kind::Symbol
            | Kind::Result
            | Kind::Option
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{FunctionProto, ProductFieldRef, ProductMetadata};

    fn unit_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        chunk
    }

    fn error(chunk: Chunk) -> String {
        validate_chunk(chunk, &ValidationLimits::default())
            .expect_err("malformed chunk must fail validation")
            .to_string()
    }

    #[test]
    fn valid_minimal_chunk_is_opaque_and_decoded() {
        let validated = validate_chunk(unit_chunk(), &ValidationLimits::default())
            .expect("minimal chunk validates");
        assert_eq!(validated.main_instructions().len(), 2);
        assert_eq!(validated.main_instructions()[0].op(), Op::Unit);
        assert_eq!(validated.main_instructions()[1].op(), Op::Return);
    }

    #[test]
    fn all_bytes_are_decoded_even_when_unreachable() {
        let mut chunk = unit_chunk();
        chunk.main.code = vec![Op::Unit as u8, Op::Return as u8, 255];
        assert!(error(chunk).contains("unknown or retired opcode"));

        let mut truncated = unit_chunk();
        truncated.main.code = vec![Op::Unit as u8, Op::Return as u8, Op::LoadConst as u8, 0];
        assert!(error(truncated).contains("truncated"));
    }

    #[test]
    fn indexes_metadata_categories_and_capture_metadata_are_checked() {
        let mut constant = unit_chunk();
        constant.main.code = vec![Op::LoadConst as u8, 0, 0, Op::Return as u8];
        assert!(error(constant).contains("constant index"));

        let mut captures = unit_chunk();
        captures.constants.push(Constant::Proto(0));
        captures.protos.push(FunctionProto {
            name: "f".into(),
            arity: 0,
            locals: 0,
            code: vec![Op::Unit as u8, Op::Return as u8],
        });
        captures.main.code = vec![
            Op::LoadConst as u8,
            0,
            0,
            Op::MakeClosure as u8,
            1,
            0,
            Op::Return as u8,
        ];
        assert!(error(captures).contains("capture metadata"));

        let mut product = unit_chunk();
        product.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "P".into(),
            fields: vec!["x".into()],
        });
        product.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        });
        product.main.code = vec![
            Op::Unit as u8,
            Op::LoadProductField as u8,
            0,
            0,
            Op::Return as u8,
        ];
        assert!(error(product).contains("product operation category"));
    }

    #[test]
    fn cfg_stack_local_return_and_fallthrough_are_checked() {
        let mut underflow = unit_chunk();
        underflow.main.code = vec![Op::Pop as u8, Op::Unit as u8, Op::Return as u8];
        assert!(error(underflow).contains("stack underflow"));

        let mut local = unit_chunk();
        local.main.locals = 1;
        local.main.code = vec![Op::LoadLocal as u8, 0, Op::Return as u8];
        assert!(error(local).contains("not definitely initialized"));

        let mut fallthrough = unit_chunk();
        fallthrough.main.code = vec![Op::Unit as u8];
        assert!(error(fallthrough).contains("falls through"));

        let mut return_shape = unit_chunk();
        return_shape.main.code = vec![Op::Unit as u8, Op::Unit as u8, Op::Return as u8];
        assert!(error(return_shape).contains("exactly one"));

        let mut join = unit_chunk();
        join.main.code = vec![
            Op::True as u8,
            Op::JumpIfFalse as u8,
            9,
            0,
            Op::Unit as u8,
            Op::Unit as u8,
            Op::Jump as u8,
            13,
            0,
            Op::Unit as u8,
            Op::Jump as u8,
            13,
            0,
            Op::Return as u8,
        ];
        assert!(error(join).contains("stack depth"));
    }

    #[test]
    fn configured_code_table_metadata_and_constant_limits_are_enforced() {
        let code_limits = ValidationLimits {
            max_function_code_bytes: 1,
            ..ValidationLimits::default()
        };
        assert!(validate_chunk(unit_chunk(), &code_limits)
            .expect_err("code limit")
            .to_string()
            .contains("code bytes"));

        let encoded_limits = ValidationLimits {
            max_encoded_bytes: 1,
            ..ValidationLimits::default()
        };
        assert!(validate_chunk(unit_chunk(), &encoded_limits)
            .expect_err("encoded limit")
            .to_string()
            .contains("encoded bytecode"));

        let mut table = unit_chunk();
        table.constants.push(Constant::I64(1));
        let table_limits = ValidationLimits {
            max_table_entries: 0,
            ..ValidationLimits::default()
        };
        assert!(validate_chunk(table, &table_limits)
            .expect_err("table limit")
            .to_string()
            .contains("table"));

        let metadata_limits = ValidationLimits {
            max_metadata_bytes: 0,
            ..ValidationLimits::default()
        };
        assert!(validate_chunk(unit_chunk(), &metadata_limits)
            .expect_err("metadata limit")
            .to_string()
            .contains("metadata"));

        let mut data = unit_chunk();
        data.constants.push(Constant::Str("x".into()));
        let data_limits = ValidationLimits {
            max_constant_data_bytes: 0,
            ..ValidationLimits::default()
        };
        assert!(validate_chunk(data, &data_limits)
            .expect_err("constant data limit")
            .to_string()
            .contains("constant 0"));
    }

    #[test]
    fn main_arity_global_initialization_and_static_operation_categories_are_checked() {
        let mut main = unit_chunk();
        main.main.arity = 1;
        main.main.locals = 1;
        assert!(error(main).contains("main entry"));

        let mut global = unit_chunk();
        global.global_names.push("g".into());
        global.main.code = vec![Op::LoadGlobal as u8, 0, 0, Op::Return as u8];
        assert!(error(global).contains("global is not definitely initialized"));

        for (operation, category) in [
            (Op::IsSome, "Option"),
            (Op::IsOk, "Result"),
            (Op::Car, "List"),
            (Op::BufLen, "Buf"),
            (Op::SysClose, "Handle"),
            (Op::BufToStr, "Buf"),
        ] {
            let mut chunk = unit_chunk();
            chunk.main.code = vec![Op::Unit as u8, operation as u8, Op::Return as u8];
            let message = error(chunk);
            assert!(
                message.contains(category),
                "wrong category diagnostic for {operation:?}: {message}"
            );
        }
    }

    #[test]
    fn bulk_byte_opcodes_reject_malformed_type_stacks() {
        let mut read = unit_chunk();
        read.main.code = vec![
            Op::Unit as u8,
            Op::Unit as u8,
            Op::Unit as u8,
            Op::Unit as u8,
            Op::SysReadInto as u8,
            Op::Return as u8,
        ];
        assert!(error(read).contains("I64"));

        let mut from = unit_chunk();
        from.main.code = vec![Op::Unit as u8, Op::BufFromStr as u8, Op::Return as u8];
        assert!(error(from).contains("Str"));

        let mut random = unit_chunk();
        random.main.code = vec![
            Op::Unit as u8,
            Op::Unit as u8,
            Op::Unit as u8,
            Op::SysRandomFill as u8,
            Op::Return as u8,
        ];
        assert!(error(random).contains("I64"));

        let mut fsync = unit_chunk();
        fsync.main.code = vec![Op::Unit as u8, Op::SysFsync as u8, Op::Return as u8];
        assert!(error(fsync).contains("Handle"));
    }

    #[test]
    fn unreachable_operands_and_duplicate_metadata_still_fail() {
        let mut unreachable = unit_chunk();
        unreachable.main.code.extend_from_slice(&[
            Op::LoadGlobal as u8,
            0,
            0,
            Op::Unit as u8,
            Op::Return as u8,
        ]);
        assert!(error(unreachable).contains("global index"));

        let mut duplicate = unit_chunk();
        duplicate.global_names.push("same".into());
        duplicate.global_names.push("same".into());
        assert!(error(duplicate).contains("duplicate bytecode global"));
    }

    #[test]
    fn random_and_small_byte_chunks_never_panic() {
        let mut seed = 0x9e37_79b9_u32;
        for length in 0..=32_usize {
            for _ in 0..128 {
                let mut chunk = Chunk::new();
                for _ in 0..length {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    chunk.main.code.push(seed.to_le_bytes()[0]);
                }
                let _result = validate_chunk(chunk, &ValidationLimits::default());
            }
        }
    }
}
