//! Lower verified normalized typed SSA into validated-reference bytecode input.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{
    Chunk, Constant as BytecodeConstant, Error, FunctionProto, Op, ProductFieldRef,
    ProductId as BytecodeProductId, ProductMetadata as BytecodeProductMetadata, Result,
};
use lkjscript_ir::{
    BlockId, BytecodeBlockLink, BytecodeInstructionLink, BytecodeLinkMetadata, CallTarget,
    Constant, Function, FunctionBytecodeLink, FunctionId, Instruction, InstructionKind, RuntimeOp,
    SsaType, Terminator, ValueId, VerifiedProgram,
};

pub(crate) fn compile_program(verified: &VerifiedProgram) -> Result<(Chunk, BytecodeLinkMetadata)> {
    let program = verified.program();
    let mut chunk = Chunk::new();
    chunk.main.name = "main".into();
    for product in &program.products {
        if product.id.index() != Some(chunk.products.len()) {
            return Err(Error::msg(
                "SSA product IDs are inconsistent during bytecode lowering",
            ));
        }
        chunk.products.push(BytecodeProductMetadata {
            id: BytecodeProductId::new(product.id.raw()),
            name: product.name.clone(),
            fields: product
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        });
    }

    let mut globals = HashMap::new();
    let mut prototypes = HashMap::new();
    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let slot = u16::try_from(chunk.global_names.len())
            .map_err(|_| Error::msg("too many SSA functions for bytecode globals"))?;
        globals.insert(function.id, slot);
        chunk.global_names.push(function.name.clone());
        let prototype = u32::try_from(prototypes.len())
            .map_err(|_| Error::msg("too many SSA functions for bytecode prototypes"))?;
        prototypes.insert(function.id, prototype);
    }

    let mut links = Vec::with_capacity(program.functions.len());
    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let prototype = prototypes.get(&function.id).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA function {} has no bytecode prototype mapping",
                function.id.raw()
            ))
        })?;
        if usize::try_from(prototype).ok() != Some(chunk.protos.len()) {
            return Err(Error::msg("SSA prototype mapping is not dense"));
        }
        let (proto, mut link) =
            compile_function(&mut chunk, &globals, function, 0, Some(prototype))?;
        link.prototype = Some(prototype);
        chunk.protos.push(proto);
        links.push(link);
    }

    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let prototype = prototypes
            .get(&function.id)
            .copied()
            .ok_or_else(|| Error::msg("SSA closure installation has no prototype mapping"))?;
        let global = globals
            .get(&function.id)
            .copied()
            .ok_or_else(|| Error::msg("SSA closure installation has no global mapping"))?;
        let constant = add_constant(&mut chunk, BytecodeConstant::Proto(prototype))?;
        chunk.main.emit_op_u16(Op::LoadConst, constant);
        chunk.main.emit_op_u16(Op::MakeClosure, 0);
        chunk.main.emit_op_u16(Op::StoreGlobal, global);
        chunk.main.emit(Op::Pop);
    }

    let main = program
        .functions
        .get(program.main.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == program.main)
        .ok_or_else(|| Error::msg("SSA main function is missing"))?;
    let code_base = u16::try_from(chunk.main.len())
        .map_err(|_| Error::msg("bytecode main closure prelude exceeds u16"))?;
    let (main_proto, main_link) = compile_function(&mut chunk, &globals, main, code_base, None)?;
    chunk.main.locals = main_proto.locals;
    chunk.main.arity = 0;
    chunk.main.code.extend(main_proto.code);
    links.push(main_link);
    links.sort_by_key(|link| link.function);

    Ok((
        chunk,
        BytecodeLinkMetadata {
            main: program.main,
            functions: links,
        },
    ))
}

fn compile_function(
    chunk: &mut Chunk,
    globals: &HashMap<FunctionId, u16>,
    function: &Function,
    code_base: u16,
    prototype: Option<u32>,
) -> Result<(FunctionProto, FunctionBytecodeLink)> {
    let slots = allocate_locals(function)?;
    let locals = u8::try_from(slots.len()).map_err(|_| {
        Error::msg(format!(
            "SSA function {} requires {} bytecode locals; limit is 255",
            function.name,
            slots.len()
        ))
    })?;
    let arity = u8::try_from(function.signature.parameters.len())
        .map_err(|_| Error::msg("SSA function arity exceeds bytecode u8"))?;
    let proto = FunctionProto {
        name: function.name.clone(),
        arity,
        locals,
        code: Vec::new(),
    };
    let mut emitter = Emitter {
        chunk,
        globals,
        function,
        slots,
        code_base,
        proto,
        block_offsets: HashMap::new(),
        patches: Vec::new(),
        block_links: Vec::new(),
        instruction_links: Vec::new(),
    };
    for block in &function.blocks {
        let offset = emitter.offset()?;
        emitter.block_offsets.insert(block.id, offset);
        emitter.block_links.push(BytecodeBlockLink {
            block: block.id,
            offset: u32::from(offset),
        });
        let tail_call = block.instructions.last().and_then(|instruction| {
            if matches!(&instruction.kind, InstructionKind::Call { .. })
                && tail_path_returns(function, &block.terminator, instruction.id)
            {
                Some(instruction.id)
            } else {
                None
            }
        });
        for instruction in &block.instructions {
            let offset = emitter.offset()?;
            emitter.instruction_links.push(BytecodeInstructionLink {
                value: instruction.id,
                offset: u32::from(offset),
            });
            emitter.emit_instruction(instruction, tail_call != Some(instruction.id))?;
        }
        if tail_call.is_some() {
            emitter.proto.emit(Op::Return);
        } else {
            emitter.emit_terminator(block.id, &block.terminator)?;
        }
    }
    emitter.patch_jumps()?;
    Ok((
        emitter.proto,
        FunctionBytecodeLink {
            function: function.id,
            prototype,
            is_main: function.id == emitter.function.id && prototype.is_none(),
            blocks: emitter.block_links,
            instructions: emitter.instruction_links,
        },
    ))
}

struct Emitter<'a> {
    chunk: &'a mut Chunk,
    globals: &'a HashMap<FunctionId, u16>,
    function: &'a Function,
    slots: HashMap<ValueId, u8>,
    code_base: u16,
    proto: FunctionProto,
    block_offsets: HashMap<BlockId, u16>,
    patches: Vec<(usize, BlockId)>,
    block_links: Vec<BytecodeBlockLink>,
    instruction_links: Vec<BytecodeInstructionLink>,
}

impl Emitter<'_> {
    fn offset(&self) -> Result<u16> {
        let local = u16::try_from(self.proto.len())
            .map_err(|_| Error::msg("bytecode function offset exceeds u16"))?;
        self.code_base
            .checked_add(local)
            .ok_or_else(|| Error::msg("bytecode function offset exceeds u16"))
    }

    fn slot(&self, value: ValueId) -> Result<u8> {
        self.slots.get(&value).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA value {} has no bytecode local slot",
                value.raw()
            ))
        })
    }

    fn load(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        self.proto.emit_op_u8(Op::LoadLocal, slot);
        Ok(())
    }

    fn store_result(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        self.proto.emit_op_u8(Op::StoreLocal, slot);
        self.proto.emit(Op::Pop);
        Ok(())
    }

    fn emit_instruction(&mut self, instruction: &Instruction, store_result: bool) -> Result<()> {
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.emit_constant(constant)?,
            InstructionKind::Copy(value) => self.load(*value)?,
            InstructionKind::FunctionRef(function) => {
                let global = self.global(*function)?;
                self.proto.emit_op_u16(Op::LoadGlobal, global);
            }
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.load(*argument)?;
                }
                self.proto.emit(runtime_opcode(*operation));
            }
            InstructionKind::Call {
                target, arguments, ..
            } => {
                for argument in arguments {
                    self.load(*argument)?;
                }
                match target {
                    CallTarget::Direct(function) => {
                        let global = self.global(*function)?;
                        self.proto.emit_op_u16(Op::LoadGlobal, global);
                    }
                    CallTarget::Indirect(value) => self.load(*value)?,
                }
                let arity = u8::try_from(arguments.len())
                    .map_err(|_| Error::msg("SSA call arity exceeds bytecode u8"))?;
                self.proto.emit_op_u8(Op::Call, arity);
            }
            InstructionKind::ProductValue { product, fields } => {
                for field in fields {
                    self.load(*field)?;
                }
                self.proto.emit_op_u16(Op::MakeProduct, product.raw());
            }
            InstructionKind::ProductField {
                product,
                field,
                value,
            } => {
                self.load(*value)?;
                let descriptor = intern_product_field(self.chunk, product.raw(), *field)?;
                self.proto.emit_op_u16(Op::LoadProductField, descriptor);
            }
            InstructionKind::WithProductField {
                product,
                field,
                value,
                replacement,
            } => {
                self.load(*value)?;
                self.load(*replacement)?;
                let descriptor = intern_product_field(self.chunk, product.raw(), *field)?;
                self.proto.emit_op_u16(Op::WithProductField, descriptor);
            }
        }
        if store_result {
            self.store_result(instruction.id)?;
        }
        Ok(())
    }

    fn emit_constant(&mut self, constant: &Constant) -> Result<()> {
        match constant {
            Constant::Unit => self.proto.emit(Op::Unit),
            Constant::Bool(false) => self.proto.emit(Op::False),
            Constant::Bool(true) => self.proto.emit(Op::True),
            Constant::I64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::I64(*value))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::F64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::F64(*value))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::Str(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Str(value.clone()))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::Symbol(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Symbol(value.clone()))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::EmptyList => self.proto.emit(Op::EmptyList),
            Constant::None => self.proto.emit(Op::OptionNone),
        }
        Ok(())
    }

    fn emit_terminator(&mut self, block: BlockId, terminator: &Terminator) -> Result<()> {
        match terminator {
            Terminator::Branch { target, arguments } => {
                self.emit_edge_arguments(*target, arguments)?;
                self.emit_jump(Op::Jump, *target);
            }
            Terminator::ConditionalBranch {
                condition,
                true_target,
                true_arguments,
                false_target,
                false_arguments,
            } => {
                self.load(*condition)?;
                self.proto.emit(Op::JumpIfFalse);
                let false_patch = self.proto.len();
                self.proto.emit_u16(0);
                self.emit_edge_arguments(*true_target, true_arguments)?;
                self.emit_jump(Op::Jump, *true_target);
                let false_offset = self.offset()?;
                self.patch_at(false_patch, false_offset)?;
                self.emit_edge_arguments(*false_target, false_arguments)?;
                self.emit_jump(Op::Jump, *false_target);
            }
            Terminator::Return(value) => {
                self.load(*value)?;
                self.proto.emit(Op::Return);
            }
            Terminator::Trap { message } => {
                let diagnostic = add_constant(self.chunk, BytecodeConstant::Str(message.clone()))?;
                self.proto.emit_op_u16(Op::Trap, diagnostic);
            }
            Terminator::Exit { code } => {
                self.load(*code)?;
                self.proto.emit(Op::Exit);
            }
            Terminator::Outcome { outcome, .. } => {
                return Err(Error::msg(format!(
                    "SSA structured outcome {outcome:?} has no source bytecode representation"
                )));
            }
        }
        if self.proto.is_empty() {
            return Err(Error::msg(format!(
                "SSA block {} emitted no bytecode",
                block.raw()
            )));
        }
        Ok(())
    }

    fn emit_edge_arguments(&mut self, target: BlockId, arguments: &[ValueId]) -> Result<()> {
        let target_block = self
            .function
            .blocks
            .iter()
            .find(|block| block.id == target)
            .ok_or_else(|| Error::msg("SSA bytecode edge target is missing"))?;
        if target_block.parameters.len() != arguments.len() {
            return Err(Error::msg("SSA bytecode edge argument count mismatch"));
        }
        for argument in arguments {
            self.load(*argument)?;
        }
        for parameter in target_block.parameters.iter().rev() {
            let slot = self.slot(parameter.id)?;
            self.proto.emit_op_u8(Op::StoreLocal, slot);
            self.proto.emit(Op::Pop);
        }
        Ok(())
    }

    fn emit_jump(&mut self, operation: Op, target: BlockId) {
        self.proto.emit(operation);
        let patch = self.proto.len();
        self.proto.emit_u16(0);
        self.patches.push((patch, target));
    }

    fn patch_jumps(&mut self) -> Result<()> {
        let patches = std::mem::take(&mut self.patches);
        for (patch, target) in patches {
            let offset = self.block_offsets.get(&target).copied().ok_or_else(|| {
                Error::msg(format!(
                    "SSA jump target block {} was not emitted",
                    target.raw()
                ))
            })?;
            self.patch_at(patch, offset)?;
        }
        Ok(())
    }

    fn patch_at(&mut self, patch: usize, offset: u16) -> Result<()> {
        let end = patch
            .checked_add(2)
            .ok_or_else(|| Error::msg("bytecode jump patch overflow"))?;
        let bytes = self
            .proto
            .code
            .get_mut(patch..end)
            .ok_or_else(|| Error::msg("bytecode jump patch is out of range"))?;
        bytes.copy_from_slice(&offset.to_le_bytes());
        Ok(())
    }

    fn global(&self, function: FunctionId) -> Result<u16> {
        self.globals.get(&function).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA function {} has no bytecode closure slot",
                function.raw()
            ))
        })
    }
}

fn tail_path_returns(function: &Function, terminator: &Terminator, value: ValueId) -> bool {
    match terminator {
        Terminator::Return(returned) => *returned == value,
        Terminator::Branch { target, arguments } => {
            follow_empty_tail_path(function, *target, arguments, value, &mut HashSet::new())
        }
        _ => false,
    }
}

fn follow_empty_tail_path(
    function: &Function,
    target: BlockId,
    arguments: &[ValueId],
    value: ValueId,
    visited: &mut HashSet<BlockId>,
) -> bool {
    if !visited.insert(target) {
        return false;
    }
    let Some(block) = function.blocks.iter().find(|block| block.id == target) else {
        return false;
    };
    if !block.instructions.is_empty() || block.parameters.len() != arguments.len() {
        return false;
    }
    let substitutions: HashMap<ValueId, ValueId> = block
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.id, *argument))
        .collect();
    let resolve = |candidate: ValueId| substitutions.get(&candidate).copied().unwrap_or(candidate);
    match &block.terminator {
        Terminator::Return(returned) => resolve(*returned) == value,
        Terminator::Branch { target, arguments } => {
            let arguments: Vec<ValueId> = arguments
                .iter()
                .map(|argument| resolve(*argument))
                .collect();
            follow_empty_tail_path(function, *target, &arguments, value, visited)
        }
        _ => false,
    }
}

fn allocate_locals(function: &Function) -> Result<HashMap<ValueId, u8>> {
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let mut value_types = HashMap::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            value_types.insert(parameter.id, parameter.ty.clone());
        }
        for instruction in &block.instructions {
            value_types.insert(instruction.id, instruction.ty.clone());
        }
    }

    let mut uses = HashMap::new();
    let mut definitions = HashMap::new();
    for block in &function.blocks {
        let mut block_uses = HashSet::new();
        let mut block_definitions: HashSet<ValueId> = block
            .parameters
            .iter()
            .map(|parameter| parameter.id)
            .collect();
        for instruction in &block.instructions {
            for operand in instruction.kind.operands() {
                if !block_definitions.contains(&operand) {
                    block_uses.insert(operand);
                }
            }
            block_definitions.insert(instruction.id);
        }
        for operand in block.terminator.operands() {
            if !block_definitions.contains(&operand) {
                block_uses.insert(operand);
            }
        }
        uses.insert(block.id, block_uses);
        definitions.insert(block.id, block_definitions);
    }

    let mut live_in: HashMap<BlockId, HashSet<ValueId>> = function
        .blocks
        .iter()
        .map(|block| (block.id, HashSet::new()))
        .collect();
    let mut live_out = live_in.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for block in function.blocks.iter().rev() {
            let mut next_out = HashSet::new();
            for successor in bytecode_successors(&block.terminator) {
                if let Some(successor_live) = live_in.get(&successor) {
                    next_out.extend(successor_live);
                }
            }
            let mut next_in = uses.get(&block.id).cloned().unwrap_or_default();
            let block_definitions = definitions.get(&block.id).cloned().unwrap_or_default();
            next_in.extend(
                next_out
                    .iter()
                    .copied()
                    .filter(|value| !block_definitions.contains(value)),
            );
            if live_out.get(&block.id) != Some(&next_out) {
                live_out.insert(block.id, next_out);
                changed = true;
            }
            if live_in.get(&block.id) != Some(&next_in) {
                live_in.insert(block.id, next_in);
                changed = true;
            }
        }
    }

    let value_count = value_types.len();
    let mut interference = vec![HashSet::new(); value_count];
    for block in &function.blocks {
        let mut live = live_out.get(&block.id).cloned().unwrap_or_default();
        live.extend(block.terminator.operands());
        for instruction in block.instructions.iter().rev() {
            add_interference(&mut interference, instruction.id, &live)?;
            live.remove(&instruction.id);
            live.extend(instruction.kind.operands());
        }
        let parameters: Vec<ValueId> = block
            .parameters
            .iter()
            .map(|parameter| parameter.id)
            .collect();
        for parameter in &parameters {
            add_interference(&mut interference, *parameter, &live)?;
        }
        for (index, left) in parameters.iter().enumerate() {
            for right in parameters.iter().skip(index.saturating_add(1)) {
                add_edge(&mut interference, *left, *right)?;
            }
        }
    }

    let mut colors: Vec<Option<usize>> = vec![None; value_count];
    let mut color_types: Vec<SsaType> = Vec::new();
    for (slot, parameter) in entry.parameters.iter().enumerate() {
        let index = parameter
            .id
            .index()
            .ok_or_else(|| Error::msg("SSA entry parameter ValueId exceeds usize"))?;
        let Some(color) = colors.get_mut(index) else {
            return Err(Error::msg("SSA entry parameter ValueId is out of range"));
        };
        *color = Some(slot);
        color_types.push(parameter.ty.clone());
    }

    let mut order: Vec<ValueId> = value_types.keys().copied().collect();
    order.sort_by(|left, right| {
        let left_degree = left
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        let right_degree = right
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        right_degree.cmp(&left_degree).then_with(|| left.cmp(right))
    });
    for value in order {
        let index = value
            .index()
            .ok_or_else(|| Error::msg("SSA ValueId exceeds usize during local allocation"))?;
        if colors.get(index).copied().flatten().is_some() {
            continue;
        }
        let ty = value_types
            .get(&value)
            .ok_or_else(|| Error::msg("SSA local allocation lost a value type"))?;
        let neighbors = interference
            .get(index)
            .ok_or_else(|| Error::msg("SSA local interference metadata is inconsistent"))?;
        let color = color_types
            .iter()
            .enumerate()
            .find(|(candidate, candidate_type)| {
                *candidate_type == ty
                    && neighbors.iter().all(|neighbor| {
                        neighbor
                            .index()
                            .and_then(|index| colors.get(index))
                            .copied()
                            .flatten()
                            != Some(*candidate)
                    })
            })
            .map(|(candidate, _)| candidate)
            .unwrap_or_else(|| {
                color_types.push(ty.clone());
                color_types.len().saturating_sub(1)
            });
        let Some(destination) = colors.get_mut(index) else {
            return Err(Error::msg("SSA local color destination is out of range"));
        };
        *destination = Some(color);
    }

    if color_types.len() > usize::from(u8::MAX) {
        return Err(Error::msg(format!(
            "SSA function {} requires {} bytecode locals after liveness allocation; limit is 255",
            function.name,
            color_types.len()
        )));
    }
    let mut slots = HashMap::with_capacity(value_count);
    for (raw, color) in colors.into_iter().enumerate() {
        let value = ValueId::new(
            u32::try_from(raw).map_err(|_| Error::msg("SSA local ValueId exceeds u32"))?,
        );
        let color = color.ok_or_else(|| Error::msg("SSA value did not receive a local color"))?;
        let slot =
            u8::try_from(color).map_err(|_| Error::msg("SSA bytecode local color exceeds u8"))?;
        slots.insert(value, slot);
    }
    Ok(slots)
}

fn add_interference(
    interference: &mut [HashSet<ValueId>],
    definition: ValueId,
    live: &HashSet<ValueId>,
) -> Result<()> {
    for value in live {
        if *value != definition {
            add_edge(interference, definition, *value)?;
        }
    }
    Ok(())
}

fn add_edge(interference: &mut [HashSet<ValueId>], left: ValueId, right: ValueId) -> Result<()> {
    let left_index = left
        .index()
        .ok_or_else(|| Error::msg("SSA interference ValueId exceeds usize"))?;
    let right_index = right
        .index()
        .ok_or_else(|| Error::msg("SSA interference ValueId exceeds usize"))?;
    let Some(left_edges) = interference.get_mut(left_index) else {
        return Err(Error::msg("SSA interference ValueId is out of range"));
    };
    left_edges.insert(right);
    let Some(right_edges) = interference.get_mut(right_index) else {
        return Err(Error::msg("SSA interference ValueId is out of range"));
    };
    right_edges.insert(left);
    Ok(())
}

fn bytecode_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Branch { target, .. } => vec![*target],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}

fn add_constant(chunk: &mut Chunk, constant: BytecodeConstant) -> Result<u16> {
    let id = u16::try_from(chunk.constants.len())
        .map_err(|_| Error::msg("too many constants for bytecode u16 IDs"))?;
    chunk.constants.push(constant);
    Ok(id)
}

fn intern_product_field(chunk: &mut Chunk, product: u16, field: u8) -> Result<u16> {
    let field_ref = ProductFieldRef {
        product: BytecodeProductId::new(product),
        field,
    };
    if let Some(index) = chunk
        .product_fields
        .iter()
        .position(|existing| *existing == field_ref)
    {
        return u16::try_from(index)
            .map_err(|_| Error::msg("product field descriptor index exceeds u16"));
    }
    let index = u16::try_from(chunk.product_fields.len())
        .map_err(|_| Error::msg("too many product field descriptors"))?;
    chunk.product_fields.push(field_ref);
    Ok(index)
}

fn runtime_opcode(operation: RuntimeOp) -> Op {
    match operation {
        RuntimeOp::Add => Op::Add,
        RuntimeOp::Subtract => Op::Sub,
        RuntimeOp::Multiply => Op::Mul,
        RuntimeOp::Divide => Op::Div,
        RuntimeOp::EqualValue => Op::EqualValue,
        RuntimeOp::SameObject => Op::SameObject,
        RuntimeOp::ListEqual => Op::ListEqual,
        RuntimeOp::F64BitsEqual => Op::F64BitsEqual,
        RuntimeOp::Less => Op::Lt,
        RuntimeOp::LessEqual => Op::Le,
        RuntimeOp::Greater => Op::Gt,
        RuntimeOp::GreaterEqual => Op::Ge,
        RuntimeOp::Not => Op::Not,
        RuntimeOp::Cons => Op::Cons,
        RuntimeOp::Car => Op::Car,
        RuntimeOp::Cdr => Op::Cdr,
        RuntimeOp::IsEmptyList => Op::IsEmptyList,
        RuntimeOp::Print => Op::Print,
        RuntimeOp::Flush => Op::Flush,
        RuntimeOp::ReadByte => Op::ReadByte,
        RuntimeOp::WriteByte => Op::WriteByte,
        RuntimeOp::BitAnd => Op::BitAnd,
        RuntimeOp::BitOr => Op::BitOr,
        RuntimeOp::BitXor => Op::BitXor,
        RuntimeOp::WriteStr => Op::WriteStr,
        RuntimeOp::EmptyStr => Op::EmptyStr,
        RuntimeOp::ArgCount => Op::Argc,
        RuntimeOp::Arg => Op::Arg,
        RuntimeOp::BufNew => Op::BufNew,
        RuntimeOp::BufLen => Op::BufLen,
        RuntimeOp::BufRef => Op::BufRef,
        RuntimeOp::BufSet => Op::BufSet,
        RuntimeOp::BufClone => Op::BufClone,
        RuntimeOp::BufFromStr => Op::BufFromStr,
        RuntimeOp::BufToStr => Op::BufToStr,
        RuntimeOp::BufSlice => Op::BufSlice,
        RuntimeOp::BufGetU32 => Op::BufGetU32,
        RuntimeOp::BufSetU32 => Op::BufSetU32,
        RuntimeOp::StrLen => Op::StrLen,
        RuntimeOp::StrRef => Op::StrRef,
        RuntimeOp::StrAppend => Op::StrAppend,
        RuntimeOp::StrSlice => Op::StrSlice,
        RuntimeOp::StrFromByte => Op::StrFromByte,
        RuntimeOp::StrFromI64 => Op::StrFromI64,
        RuntimeOp::StrFromF64 => Op::StrFromF64,
        RuntimeOp::StdinHandle => Op::StdinHandle,
        RuntimeOp::SysIsatty => Op::SysIsatty,
        RuntimeOp::SysClose => Op::SysClose,
        RuntimeOp::SysReadByte => Op::SysReadByte,
        RuntimeOp::SysWriteByte => Op::SysWriteByte,
        RuntimeOp::SysReadInto => Op::SysReadInto,
        RuntimeOp::SysWriteFrom => Op::SysWriteFrom,
        RuntimeOp::SysTtyGuardSave => Op::SysTtyGuardSave,
        RuntimeOp::SysTtyGuardClear => Op::SysTtyGuardClear,
        RuntimeOp::SysOpenRead => Op::SysOpenRead,
        RuntimeOp::SysOpenWrite => Op::SysOpenWrite,
        RuntimeOp::SysOpenAppend => Op::SysOpenAppend,
        RuntimeOp::SysOpenCreateNew => Op::SysOpenCreateNew,
        RuntimeOp::SysOpenDir => Op::SysOpenDir,
        RuntimeOp::SysFsync => Op::SysFsync,
        RuntimeOp::SysTruncate => Op::SysTruncate,
        RuntimeOp::SysRename => Op::SysRename,
        RuntimeOp::SysRandomFill => Op::SysRandomFill,
        RuntimeOp::SysSha256 => Op::SysSha256,
        RuntimeOp::SysSqliteOpen => Op::SysSqliteOpen,
        RuntimeOp::SysSqliteClose => Op::SysSqliteClose,
        RuntimeOp::SysSqliteBusyTimeout => Op::SysSqliteBusyTimeout,
        RuntimeOp::SysSqliteExec => Op::SysSqliteExec,
        RuntimeOp::SysSqlitePrepare => Op::SysSqlitePrepare,
        RuntimeOp::SysSqliteFinalize => Op::SysSqliteFinalize,
        RuntimeOp::SysSqliteReset => Op::SysSqliteReset,
        RuntimeOp::SysSqliteClearBindings => Op::SysSqliteClearBindings,
        RuntimeOp::SysSqliteBindNull => Op::SysSqliteBindNull,
        RuntimeOp::SysSqliteBindI64 => Op::SysSqliteBindI64,
        RuntimeOp::SysSqliteBindF64 => Op::SysSqliteBindF64,
        RuntimeOp::SysSqliteBindText => Op::SysSqliteBindText,
        RuntimeOp::SysSqliteBindBytes => Op::SysSqliteBindBytes,
        RuntimeOp::SysSqliteStep => Op::SysSqliteStep,
        RuntimeOp::SysSqliteColumnCount => Op::SysSqliteColumnCount,
        RuntimeOp::SysSqliteColumnType => Op::SysSqliteColumnType,
        RuntimeOp::SysSqliteColumnI64 => Op::SysSqliteColumnI64,
        RuntimeOp::SysSqliteColumnF64 => Op::SysSqliteColumnF64,
        RuntimeOp::SysSqliteColumnText => Op::SysSqliteColumnText,
        RuntimeOp::SysSqliteColumnBytes => Op::SysSqliteColumnBytes,
        RuntimeOp::SysSqliteChanges => Op::SysSqliteChanges,
        RuntimeOp::SysSqliteLastInsertRowid => Op::SysSqliteLastInsertRowid,
        RuntimeOp::SysSqliteExtendedResultCode => Op::SysSqliteExtendedResultCode,
        RuntimeOp::SysSqliteBackup => Op::SysSqliteBackup,
        RuntimeOp::SysPathExists => Op::SysPathExists,
        RuntimeOp::SysWaitMs => Op::SysWaitMs,
        RuntimeOp::SysNowMs => Op::SysNowMs,
        RuntimeOp::SysSocket => Op::SysSocket,
        RuntimeOp::SysBind => Op::SysBind,
        RuntimeOp::SysListen => Op::SysListen,
        RuntimeOp::SysAccept => Op::SysAccept,
        RuntimeOp::SysRecv => Op::SysRecv,
        RuntimeOp::SysSend => Op::SysSend,
        RuntimeOp::SysPoll => Op::SysPoll,
        RuntimeOp::SysTtyGet => Op::SysTtyGet,
        RuntimeOp::SysTtySet => Op::SysTtySet,
        RuntimeOp::Ok => Op::OkWrap,
        RuntimeOp::Err => Op::ErrWrap,
        RuntimeOp::IsOk => Op::IsOk,
        RuntimeOp::UnwrapOk => Op::UnwrapOk,
        RuntimeOp::UnwrapErr => Op::UnwrapErr,
        RuntimeOp::Some => Op::SomeWrap,
        RuntimeOp::IsSome => Op::IsSome,
        RuntimeOp::UnwrapSome => Op::UnwrapSome,
    }
}
