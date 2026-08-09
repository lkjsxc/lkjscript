mod emitter;
mod failure;
use crate::codegen::*;
pub(in crate::codegen) use emitter::Emitter;
use failure::*;
pub(in crate::codegen) fn compile_function(
    chunk: &mut Chunk,
    globals: &HashMap<FunctionId, BytecodeGlobalId>,
    function: &Function,
    code_base: u64,
    prototype: Option<u64>,
) -> Result<(FunctionProto, FunctionBytecodeLink)> {
    let LocalAllocation { slots, metadata } = allocate_locals(function, chunk)?;
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let locals = physical_local_count(&slots)?;
    let arity = function.signature.parameters.len();
    let failure_index = FailureCodegenIndex::new(function)?;
    let (failure_cleanups, failure_cleanup_map) =
        compile_failure_cleanups(function, &slots, chunk, &failure_index)?;
    let mut proto = FunctionProto {
        name: function.name.clone(),
        arity,
        locals,
        memory_plan: chunk.memory_plan,
        memory_witness_parameters: function
            .signature
            .memory_witness_parameters
            .iter()
            .map(|requirement| {
                let parameter = function
                    .signature
                    .type_parameters
                    .iter()
                    .position(|name| name == &requirement.parameter)
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| Error::msg("SSA memory witness parameter exceeds u64"))?
                    .ok_or_else(|| Error::msg("SSA memory witness parameter is not canonical"))?;
                Ok(lkjscript_core::MemoryWitnessParameter {
                    parameter,
                    operations: requirement.operations.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        call_witnesses: Vec::new(),
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
                        usize::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA structural parameter PlaceId exceeds host usize")
                        })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?,
        parameter_type_variables: function
            .signature
            .parameters
            .iter()
            .map(|ty| type_variable(&function.signature, ty))
            .collect::<Result<Vec<_>>>()?,
        parameter_copy_kinds: function
            .signature
            .parameters
            .iter()
            .map(copy_parameter_kind)
            .collect(),
        return_copy_kind: copy_parameter_kind(&function.signature.result),
        parameter_region_products: function
            .signature
            .parameters
            .iter()
            .map(|ty| region_product(chunk, ty))
            .collect(),
        return_region_product: region_product(chunk, &function.signature.result),
        return_structural: structural_owner_representation(chunk, &function.signature.result),
        return_type_variable: type_variable(
            &function.signature,
            function.signature.result.as_ref(),
        )?,
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
                        usize::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA resource parameter PlaceId exceeds host usize")
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
                        usize::try_from(place.raw()).map_err(|_| {
                            Error::msg("SSA parameter owner PlaceId exceeds host usize")
                        })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?,
        return_unique: unique_value_kind(&function.signature.result),
        unique_places: function.places.len(),
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: Vec::new(),
    };
    let block_count = function.blocks.len();
    let instruction_count = function.blocks.iter().try_fold(0_usize, |count, block| {
        count
            .checked_add(block.instructions.len())
            .ok_or_else(|| Error::msg("SSA bytecode instruction count exceeds host usize"))
    })?;
    let range_capacity = block_count
        .checked_add(instruction_count)
        .ok_or_else(|| Error::msg("bytecode failure-range count exceeds host usize"))?;
    let patch_capacity = block_count
        .checked_mul(2)
        .ok_or_else(|| Error::msg("bytecode jump-patch count exceeds host usize"))?;
    proto
        .failure_cleanup_ranges
        .try_reserve_exact(range_capacity)
        .map_err(|_| Error::host("bytecode failure-range reservation failed"))?;
    proto.try_reserve_code(range_capacity)?;
    let mut block_offsets = HashMap::new();
    block_offsets
        .try_reserve(block_count)
        .map_err(|_| Error::host("bytecode block-offset reservation failed"))?;
    let mut patches = Vec::new();
    patches
        .try_reserve_exact(patch_capacity)
        .map_err(|_| Error::host("bytecode jump-patch reservation failed"))?;
    let mut block_links = Vec::new();
    block_links
        .try_reserve_exact(block_count)
        .map_err(|_| Error::host("bytecode block-link reservation failed"))?;
    let mut instruction_links = Vec::new();
    instruction_links
        .try_reserve_exact(instruction_count)
        .map_err(|_| Error::host("bytecode instruction-link reservation failed"))?;
    let nonowned_structural_values = collect_nonowned_structural_values(function);
    let mut emitter = Emitter {
        chunk,
        globals,
        function,
        slots,
        local_metadata: metadata,
        nonowned_structural_values,
        code_base,
        proto,
        block_offsets,
        patches,
        block_links,
        instruction_links,
        failure_cleanup_map,
        failure_cleanups,
        failure_index,
    };
    emit_blocks(&mut emitter)?;
    emitter.patch_jumps()?;
    emitter.proto.failure_cleanups = std::mem::take(&mut emitter.failure_cleanups).into_nodes();
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

fn physical_local_count(slots: &HashMap<ValueId, usize>) -> Result<usize> {
    slots
        .values()
        .copied()
        .max()
        .map(|slot| {
            slot.checked_add(1)
                .ok_or_else(|| Error::host("bytecode local count overflow"))
        })
        .transpose()
        .map(|count| count.unwrap_or(0))
}

include!("model/helpers.rs");

#[cfg(test)]
mod tests {
    use super::physical_local_count;
    use lkjscript_ir::ValueId;
    use std::collections::HashMap;

    #[test]
    fn bytecode_frame_size_tracks_physical_colors_not_ssa_value_count() -> lkjscript_core::Result<()>
    {
        let slots = HashMap::from([
            (ValueId::new(0), 0),
            (ValueId::new(1), 1),
            (ValueId::new(2), 0),
            (ValueId::new(3), 2),
            (ValueId::new(4), 1),
        ]);
        assert_eq!(physical_local_count(&slots)?, 3);
        assert_eq!(physical_local_count(&HashMap::new())?, 0);
        Ok(())
    }
}
