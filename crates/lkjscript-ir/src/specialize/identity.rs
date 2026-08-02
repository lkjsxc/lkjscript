use crate::{
    BlockId, Function, GenericInstantiation, IrError, SsaType, StructuralMemoryMetadata, Terminator,
};
use lkjscript_contracts::MemoryWitnessOperation;

pub(super) fn specialize_identity(
    function: &mut Function,
    instantiation: &GenericInstantiation,
    memory: &StructuralMemoryMetadata,
) -> crate::Result<()> {
    let [parameter] = function.signature.type_parameters.as_slice() else {
        return Err(IrError::new(
            "native transport specialization requires one type parameter",
        ));
    };
    let [requirement] = function.signature.memory_witness_parameters.as_slice() else {
        return Err(IrError::new(
            "native transport specialization requires one hidden witness",
        ));
    };
    let [declared_parameter] = function.signature.parameters.as_slice() else {
        return Err(IrError::new(
            "native transport specialization requires one value parameter",
        ));
    };
    let place_matches = match function.places.as_slice() {
        [] => true,
        [place] => place.ty == *declared_parameter && place.drop_glue.is_none(),
        _ => false,
    };
    if requirement.parameter != *parameter
        || requirement.operations != [MemoryWitnessOperation::Transport]
        || declared_parameter != &SsaType::TypeParameter(parameter.clone())
        || function.signature.result.as_ref() != declared_parameter
        || !place_matches
        || function.blocks.len() != 1
        || function.entry != BlockId::new(0)
    {
        return Err(IrError::new(
            "native transport specialization target is not an exact identity body",
        ));
    }
    let block = &mut function.blocks[0];
    let [block_parameter] = block.parameters.as_mut_slice() else {
        return Err(IrError::new("native identity block parameter is missing"));
    };
    if !block.instructions.is_empty()
        || block_parameter.ty != *declared_parameter
        || block_parameter.owner_place.is_some()
        || block.terminator != Terminator::Return(block_parameter.id)
    {
        return Err(IrError::new(
            "native transport specialization body is not identity",
        ));
    }
    let [substitution] = instantiation.substitutions.as_slice() else {
        return Err(IrError::new(
            "native transport specialization requires one exact substitution",
        ));
    };
    let [binding] = instantiation.memory_witnesses.as_slice() else {
        return Err(IrError::new(
            "native transport specialization requires one exact witness binding",
        ));
    };
    let witness = memory
        .witness(binding.witness)
        .ok_or_else(|| IrError::new("native transport specialization witness is not installed"))?;
    if substitution.parameter != *parameter
        || binding.parameter != *parameter
        || contains_type_parameter(&substitution.ty)
        || witness.ty != substitution.ty
        || requirement
            .operations
            .iter()
            .any(|operation| !witness.supports(*operation))
        || function.signature.bounds.len() != instantiation.witnesses.len()
        || function
            .signature
            .bounds
            .iter()
            .zip(&instantiation.witnesses)
            .any(|(bound, witness)| {
                bound.parameter != *parameter
                    || witness.trait_id != bound.trait_id
                    || witness.ty != substitution.ty
            })
    {
        return Err(IrError::new(
            "native transport specialization substitution or witness identity mismatch",
        ));
    }
    function.signature.type_parameters.clear();
    function.signature.bounds.clear();
    function.signature.memory_witness_parameters.clear();
    function.signature.parameters = vec![substitution.ty.clone()];
    *function.signature.result = substitution.ty.clone();
    if let Some(place) = function.places.first_mut() {
        place.ty = substitution.ty.clone();
    }
    block_parameter.ty = substitution.ty.clone();
    Ok(())
}

fn contains_type_parameter(ty: &SsaType) -> bool {
    match ty {
        SsaType::TypeParameter(_) => true,
        SsaType::Enum { arguments, .. } => arguments.iter().any(contains_type_parameter),
        SsaType::List(element) => contains_type_parameter(element),
        SsaType::Function(signature) => {
            !signature.type_parameters.is_empty()
                || signature.parameters.iter().any(contains_type_parameter)
                || contains_type_parameter(&signature.result)
        }
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Str
        | SsaType::Symbol
        | SsaType::Bytes
        | SsaType::ByteVector
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Path
        | SsaType::Capability(_)
        | SsaType::Resource(_)
        | SsaType::StructuralDestination(_)
        | SsaType::Product(_) => false,
    }
}
