mod identity;
mod instances;

use std::collections::BTreeMap;

use crate::{
    verify, CallTarget, FunctionId, GenericInstantiation, InstructionKind, IrError, VerifiedProgram,
};
use identity::specialize_identity;
use instances::{checked_instance_count, record_instance, NativeInstances};
pub use instances::{
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS, MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION,
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeSpecializationStats {
    pub functions: u32,
    pub calls: u32,
}

pub fn specialize_native_transport(
    input: &VerifiedProgram,
) -> crate::Result<(VerifiedProgram, NativeSpecializationStats)> {
    let mut instances = NativeInstances::new();
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
                if instantiation.memory_witnesses.is_empty()
                    || residual_native_witness_function(input.program(), *target)
                {
                    continue;
                }
                calls = calls
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("native specialization call count overflow"))?;
                record_instance(&mut instances, *target, instantiation.clone())?;
            }
        }
    }
    let instance_count = checked_instance_count(&instances)?;

    let mut program = input.program().clone();
    let mut replacements = BTreeMap::<(FunctionId, GenericInstantiation), FunctionId>::new();
    let mut specialized_originals = Vec::with_capacity(instances.len());
    let mut specialized_functions =
        Vec::with_capacity(instance_count.saturating_sub(instances.len()));
    for (target, target_instances) in &instances {
        let original = program
            .functions
            .get(target.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == *target)
            .cloned()
            .ok_or_else(|| IrError::new("native specialization target is missing"))?;
        for (ordinal, instantiation) in target_instances.iter().enumerate() {
            let mut function = original.clone();
            let specialized_id = if ordinal == 0 {
                *target
            } else {
                let next_index = program
                    .functions
                    .len()
                    .checked_add(specialized_functions.len())
                    .ok_or_else(|| {
                        IrError::new("native specialization function identity overflow")
                    })?;
                let id = FunctionId::new(u32::try_from(next_index).map_err(|_| {
                    IrError::new("native specialization function identity overflow")
                })?);
                function.id = id;
                function.name = format!("{}$native-transport-{ordinal}", original.name);
                id
            };
            specialize_identity(&mut function, instantiation, &program.memory)?;
            replacements.insert((*target, instantiation.clone()), specialized_id);
            if ordinal == 0 {
                specialized_originals.push((target.index().unwrap_or(usize::MAX), function));
            } else {
                specialized_functions.push(function);
            }
        }
    }
    for (index, function) in specialized_originals {
        program.functions[index] = function;
    }
    program.functions.extend(specialized_functions);

    let residual_functions = program
        .functions
        .iter()
        .filter(|function| residual_native_witness_function(&program, function.id))
        .map(|function| function.id)
        .collect::<std::collections::BTreeSet<_>>();
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
                let Some(facts) = instantiation.as_ref() else {
                    continue;
                };
                if facts.memory_witnesses.is_empty() || residual_functions.contains(target) {
                    continue;
                }
                let specialized_target = replacements
                    .get(&(*target, facts.clone()))
                    .copied()
                    .ok_or_else(|| {
                        IrError::new("native transport specialization rewrite identity mismatch")
                    })?;
                *target = specialized_target;
                *instantiation = None;
            }
        }
    }
    let specialized = verify(program)?;
    Ok((
        specialized,
        NativeSpecializationStats {
            functions: u32::try_from(instance_count).unwrap_or(u32::MAX),
            calls,
        },
    ))
}

fn residual_native_witness_function(program: &crate::Program, function: FunctionId) -> bool {
    program
        .functions
        .get(function.index().unwrap_or(usize::MAX))
        .filter(|item| item.id == function)
        .is_some_and(|item| {
            !item.signature.memory_witness_parameters.is_empty()
                && item
                    .signature
                    .memory_witness_parameters
                    .iter()
                    .any(|requirement| {
                        requirement
                            .operations
                            .contains(&lkjscript_contracts::MemoryWitnessOperation::Compare)
                            || (requirement.operations.contains(
                                &lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
                            ) && requirement
                                .operations
                                .contains(&lkjscript_contracts::MemoryWitnessOperation::Dispose))
                    })
        })
}
