mod emitter;
mod failure;
pub(in crate::codegen) use emitter::Emitter;
use failure::*;

use crate::codegen::*;

pub(in crate::codegen) fn compile_function(
    chunk: &mut Chunk,
    globals: &HashMap<FunctionId, u16>,
    function: &Function,
    code_base: u16,
    prototype: Option<u32>,
) -> Result<(FunctionProto, FunctionBytecodeLink)> {
    let slots = allocate_locals(function, chunk)?;
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let locals = u8::try_from(slots.len()).map_err(|_| {
        Error::msg(format!(
            "SSA function {} requires {} bytecode locals; limit is 255",
            function.name,
            slots.len()
        ))
    })?;
    let arity = u8::try_from(function.signature.parameters.len())
        .map_err(|_| Error::msg("SSA function arity exceeds bytecode u8"))?;
    let (failure_cleanups, failure_cleanup_map) =
        compile_failure_cleanups(function, &slots, chunk)?;
    let proto = FunctionProto {
        name: function.name.clone(),
        arity,
        locals,
        memory_plan: chunk.memory_plan,
        parameter_structurals: entry
            .parameters
            .iter()
            .zip(&function.signature.parameters)
            .map(|(_, ty)| structural_owner_representation(chunk, ty))
            .collect(),
        parameter_structural_places: entry
            .parameters
            .iter()
            .zip(&function.signature.parameters)
            .map(|(parameter, ty)| {
                if structural_owner_representation(chunk, ty).is_none() {
                    return Ok(None);
                }
                parameter
                    .owner_place
                    .map(|place| {
                        u8::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA structural parameter PlaceId exceeds bytecode u8")
                        })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?,
        return_structural: structural_owner_representation(chunk, &function.signature.result),
        parameter_resources: function
            .signature
            .parameters
            .iter()
            .map(|ty| match ty {
                SsaType::Resource(kind) => Some(*kind),
                _ => None,
            })
            .collect(),
        parameter_resource_places: entry
            .parameters
            .iter()
            .zip(&function.signature.parameters)
            .map(|(parameter, ty)| {
                if !matches!(ty, SsaType::Resource(_)) {
                    return Ok(None);
                }
                parameter
                    .owner_place
                    .map(|place| {
                        u8::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA resource parameter PlaceId exceeds bytecode u8")
                        })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?,
        return_resource: resource_return_kind(&function.signature.result),
        parameter_uniques: function
            .signature
            .parameters
            .iter()
            .map(unique_value_kind)
            .collect(),
        parameter_unique_places: entry
            .parameters
            .iter()
            .zip(&function.signature.parameters)
            .map(|(parameter, ty)| {
                if !matches!(ty, SsaType::Bytes | SsaType::ByteVector) {
                    return Ok(None);
                }
                parameter
                    .owner_place
                    .map(|place| {
                        u8::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA parameter owner PlaceId exceeds bytecode u8")
                        })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?,
        return_unique: unique_value_kind(&function.signature.result),
        unique_places: u8::try_from(function.places.len())
            .map_err(|_| Error::msg("SSA unique place count exceeds bytecode u8"))?,
        failure_cleanups,
        failure_cleanup_ranges: Vec::new(),
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
        failure_cleanup_map,
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
                && instruction.metadata.failure_cleanup.is_none()
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
            let end = emitter.offset()?;
            let unentered_plan = emitter.intern_unentered_cleanup(instruction)?;
            emitter.record_failure_range(
                offset,
                end,
                instruction.metadata.failure_cleanup.map(|id| id.raw()),
                unentered_plan,
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
                block.metadata.failure_cleanup.map(|id| id.raw()),
                None,
            )?;
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

include!("model/helpers.rs");
