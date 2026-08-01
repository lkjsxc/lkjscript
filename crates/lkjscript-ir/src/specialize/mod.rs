use std::collections::BTreeMap;

use crate::{
    verify, BlockId, CallTarget, Function, FunctionId, GenericInstantiation, InstructionKind,
    IrError, SsaType, Terminator, VerifiedProgram,
};
use lkjscript_contracts::MemoryWitnessOperation;

pub const MAX_NATIVE_TRANSPORT_SPECIALIZATIONS: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeSpecializationStats {
    pub functions: u32,
    pub calls: u32,
}

pub fn specialize_native_transport(
    input: &VerifiedProgram,
) -> crate::Result<(VerifiedProgram, NativeSpecializationStats)> {
    let mut instances = BTreeMap::<FunctionId, GenericInstantiation>::new();
    let mut calls = 0u32;
    for function in &input.program().functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let InstructionKind::Call {
                    target: CallTarget::Direct(target),
                    instantiation: Some(instantiation),
                    ..
                } = &instruction.kind
                else {
                    continue;
                };
                if instantiation.memory_witnesses.is_empty() {
                    continue;
                }
                calls = calls
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("native specialization call count overflow"))?;
                if let Some(existing) = instances.get(target) {
                    if existing != instantiation {
                        return Err(IrError::new(
                            "native transport specialization has multiple instances",
                        ));
                    }
                } else {
                    instances.insert(*target, instantiation.clone());
                }
            }
        }
    }
    if instances.len() > MAX_NATIVE_TRANSPORT_SPECIALIZATIONS {
        return Err(IrError::new(
            "native transport specialization budget exceeded",
        ));
    }
    let mut program = input.program().clone();
    for (target, instantiation) in &instances {
        let function = program
            .functions
            .get_mut(target.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == *target)
            .ok_or_else(|| IrError::new("native specialization target is missing"))?;
        specialize_identity(function, instantiation)?;
    }
    for function in &mut program.functions {
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let InstructionKind::Call {
                    target: CallTarget::Direct(target),
                    instantiation,
                    ..
                } = &mut instruction.kind
                else {
                    continue;
                };
                if instances.contains_key(target) {
                    *instantiation = None;
                }
            }
        }
    }
    let specialized = verify(program)?;
    Ok((
        specialized,
        NativeSpecializationStats {
            functions: u32::try_from(instances.len()).unwrap_or(u32::MAX),
            calls,
        },
    ))
}

fn specialize_identity(
    function: &mut Function,
    instantiation: &GenericInstantiation,
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
    let substitution = instantiation
        .substitutions
        .iter()
        .find(|item| item.parameter == *parameter)
        .ok_or_else(|| IrError::new("native transport substitution is missing"))?;
    if matches!(substitution.ty, SsaType::TypeParameter(_))
        || !instantiation
            .memory_witnesses
            .iter()
            .any(|binding| binding.parameter == *parameter)
    {
        return Err(IrError::new(
            "native transport specialization lacks a concrete witness binding",
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
