mod emitter;
mod failure;
use crate::codegen::*;
pub(in crate::codegen) use emitter::Emitter;
use failure::*;
pub(in crate::codegen) fn compile_function(
    chunk: &mut Chunk,
    globals: &HashMap<FunctionId, u16>,
    function: &Function,
    code_base: u64,
    prototype: Option<u32>,
) -> Result<(FunctionProto, FunctionBytecodeLink)> {
    let slots = allocate_locals(function, chunk)?;
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let locals = slots.len();
    let arity = function.signature.parameters.len();
    let failure_index = FailureCodegenIndex::new(function)?;
    let (failure_cleanups, failure_cleanup_map) =
        compile_failure_cleanups(function, &slots, chunk, &failure_index)?;
    let proto = FunctionProto {
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
                    .and_then(|index| u16::try_from(index).ok())
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

include!("model/helpers.rs");
