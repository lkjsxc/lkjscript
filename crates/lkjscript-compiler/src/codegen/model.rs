use crate::codegen::*;

pub(in crate::codegen) fn compile_function(
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
        parameter_resources: function
            .signature
            .parameters
            .iter()
            .map(|ty| match ty {
                SsaType::Resource(kind) => Some(*kind),
                _ => None,
            })
            .collect(),
        return_resource: resource_return_kind(&function.signature.result),
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

fn resource_return_kind(ty: &SsaType) -> Option<ResourceReturnKind> {
    match ty {
        SsaType::Resource(kind) => Some(ResourceReturnKind::Resource(*kind)),
        SsaType::Enum { id, arguments }
            if id.bytes() == lkjscript_core::RESULT_ID
                && matches!(arguments.as_slice(), [SsaType::Resource(_), _]) =>
        {
            let [SsaType::Resource(kind), _] = arguments.as_slice() else {
                return None;
            };
            Some(ResourceReturnKind::Result(*kind))
        }
        _ => None,
    }
}

pub(in crate::codegen) struct Emitter<'a> {
    pub(in crate::codegen) chunk: &'a mut Chunk,
    pub(in crate::codegen) globals: &'a HashMap<FunctionId, u16>,
    pub(in crate::codegen) function: &'a Function,
    pub(in crate::codegen) slots: HashMap<ValueId, u8>,
    pub(in crate::codegen) code_base: u16,
    pub(in crate::codegen) proto: FunctionProto,
    pub(in crate::codegen) block_offsets: HashMap<BlockId, u16>,
    pub(in crate::codegen) patches: Vec<(usize, BlockId)>,
    pub(in crate::codegen) block_links: Vec<BytecodeBlockLink>,
    pub(in crate::codegen) instruction_links: Vec<BytecodeInstructionLink>,
}
