fn emit_blocks(emitter: &mut Emitter<'_>) -> Result<()> {
    let function = emitter.function;
    for block in &function.blocks {
        let offset = emitter.offset()?;
        emitter.block_offsets.insert(block.id, offset);
        emitter.block_links.push(BytecodeBlockLink {
            block: block.id,
            offset: u32::try_from(offset)
                .map_err(|_| Error::msg("bytecode block link offset exceeds u32"))?,
        });
        let tail_call = tail_call_value(function, block);
        for instruction in &block.instructions {
            let offset = emitter.offset()?;
            emitter.instruction_links.push(BytecodeInstructionLink {
                value: instruction.id,
                offset: u32::try_from(offset)
                    .map_err(|_| Error::msg("bytecode instruction link offset exceeds u32"))?,
            });
            emitter.emit_instruction(instruction, tail_call != Some(instruction.id))?;
            let end = emitter.offset()?;
            let unentered = emitter.intern_unentered_cleanup(instruction)?;
            emitter.record_failure_range(
                offset,
                end,
                instruction.metadata.failure_cleanup,
                unentered,
            )?;
        }
        if tail_call.is_some() {
            emitter.proto.emit(Op::Return);
        } else {
            let offset = emitter.offset()?;
            emitter.emit_terminator(block.id, &block.terminator)?;
            let end = emitter.offset()?;
            emitter.record_failure_range(
                offset,
                end,
                block.metadata.failure_cleanup,
                None,
            )?;
        }
    }
    Ok(())
}

fn tail_call_value(function: &Function, block: &lkjscript_ir::Block) -> Option<ValueId> {
    block.instructions.last().and_then(|instruction| {
        (matches!(&instruction.kind, InstructionKind::Call { .. })
            && instruction.metadata.failure_cleanup.is_none()
            && tail_path_returns(function, &block.terminator, instruction.id))
        .then_some(instruction.id)
    })
}

fn unique_value_kind(ty: &SsaType) -> Option<UniqueValueKind> {
    match ty {
        SsaType::Bytes => Some(UniqueValueKind::Bytes),
        SsaType::ByteVector => Some(UniqueValueKind::ByteVector),
        SsaType::ByteSlice => Some(UniqueValueKind::ByteSlice),
        SsaType::ByteSliceMut => Some(UniqueValueKind::ByteSliceMut),
        _ => None,
    }
}

fn region_product(chunk: &Chunk, ty: &SsaType) -> Option<BytecodeProductId> {
    let SsaType::Product(id) = ty else {
        return None;
    };
    chunk
        .products
        .get(id.index()?)
        .filter(|product| product.region)
        .map(|product| product.id)
}

fn copy_parameter_kind(ty: &SsaType) -> Option<lkjscript_core::StructuralKind> {
    Some(match ty {
        SsaType::Unit => lkjscript_core::StructuralKind::Unit,
        SsaType::Bool => lkjscript_core::StructuralKind::Bool,
        SsaType::I64 => lkjscript_core::StructuralKind::I64,
        SsaType::F64 => lkjscript_core::StructuralKind::F64,
        SsaType::Symbol => lkjscript_core::StructuralKind::Static,
        _ => return None,
    })
}

fn type_variable(signature: &lkjscript_ir::Signature, ty: &SsaType) -> Result<Option<u16>> {
    let SsaType::TypeParameter(name) = ty else {
        return Ok(None);
    };
    signature
        .type_parameters
        .iter()
        .position(|candidate| candidate == name)
        .map(|index| {
            u16::try_from(index).map_err(|_| Error::msg("SSA type-variable index exceeds u16"))
        })
        .transpose()
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
