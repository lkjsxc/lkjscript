use std::collections::{HashMap, HashSet};

use crate::verify::*;
use crate::{GenericInstantiation, IrError, Program, Signature, SsaType, ValueId};

pub(crate) fn verify_resolved_signature(
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
    types: &[SsaType],
) -> crate::Result<()> {
    if !signature.type_parameters.is_empty()
        || !signature.bounds.is_empty()
        || !signature.memory_witness_parameters.is_empty()
        || signature.parameters.len() != arguments.len()
        || signature.result.as_ref() != result
    {
        return fail("SSA call has an unresolved or inconsistent signature");
    }
    for (argument, parameter) in arguments.iter().zip(&signature.parameters) {
        if value_type(types, *argument)? != parameter {
            return fail("SSA call argument type does not match resolved signature");
        }
    }
    Ok(())
}

pub(crate) fn verify_call_compatibility(
    program: &Program,
    declared: &Signature,
    resolved: &Signature,
    instantiation: Option<&GenericInstantiation>,
    caller_type_parameters: &[&str],
) -> crate::Result<()> {
    if declared.parameters.len() != resolved.parameters.len() {
        return fail("SSA call arity does not match callee");
    }
    let permitted: HashSet<&str> = declared
        .type_parameters
        .iter()
        .map(String::as_str)
        .collect();
    let mut substitutions: HashMap<&str, SsaType> = HashMap::new();
    for (declared, resolved) in declared.parameters.iter().zip(&resolved.parameters) {
        bind_type(declared, resolved, &permitted, &mut substitutions)?;
    }
    let expected_result = substitute_type(&declared.result, &substitutions);
    if expected_result != *resolved.result {
        return fail("SSA call result type does not match callee");
    }
    if declared.type_parameters.is_empty() {
        if instantiation.is_some() {
            return fail("SSA monomorphic call carries generic instantiation facts");
        }
        return Ok(());
    }
    if signature_contains_ownership(program, declared)
        || signature_contains_ownership(program, resolved)
        || substitutions
            .values()
            .any(|ty| contains_ownership_type(program, ty))
    {
        return fail("SSA ownership/reference generic instantiation is unavailable in this slice");
    }
    let instantiation = instantiation
        .ok_or_else(|| IrError::new("SSA generic call is missing instantiation facts"))?;
    if instantiation.substitutions.len() != declared.type_parameters.len() {
        return fail("SSA generic call has a non-canonical substitution count");
    }
    for (parameter, fact) in declared
        .type_parameters
        .iter()
        .zip(&instantiation.substitutions)
    {
        if fact.parameter != *parameter || substitutions.get(parameter.as_str()) != Some(&fact.ty) {
            return fail("SSA generic call substitution identity does not match inference");
        }
        verify_type(program, &fact.ty, caller_type_parameters)?;
        if contains_ownership_type(program, &fact.ty) {
            return fail(
                "SSA ownership/reference generic instantiation is unavailable in this slice",
            );
        }
    }
    if instantiation.memory_witnesses.len() != declared.memory_witness_parameters.len() {
        return fail("SSA generic call memory witness count does not match hidden parameters");
    }
    for (requirement, binding) in declared
        .memory_witness_parameters
        .iter()
        .zip(&instantiation.memory_witnesses)
    {
        let expected_type = substitutions
            .get(requirement.parameter.as_str())
            .ok_or_else(|| IrError::new("SSA memory witness parameter was not inferred"))?;
        let descriptor = program
            .memory
            .witness(binding.witness)
            .ok_or_else(|| IrError::new("SSA generic call memory witness is not installed"))?;
        if binding.parameter != requirement.parameter
            || &descriptor.ty != expected_type
            || requirement
                .operations
                .iter()
                .any(|operation| !descriptor.supports(*operation))
        {
            return fail("SSA generic call memory witness does not match type or operations");
        }
    }
    if instantiation.witnesses.len() != declared.bounds.len() {
        return fail("SSA generic call witness count does not match bounds");
    }
    let mut seen = HashSet::new();
    for (bound, witness) in declared.bounds.iter().zip(&instantiation.witnesses) {
        let expected_type = substitutions
            .get(bound.parameter.as_str())
            .ok_or_else(|| IrError::new("SSA trait bound parameter was not inferred"))?;
        if witness.trait_id != bound.trait_id || &witness.ty != expected_type {
            return fail("SSA trait witness type or trait does not match its bound");
        }
        if !seen.insert((witness.trait_id, witness.ty.clone())) {
            return fail("SSA generic call has duplicate trait witnesses");
        }
        verify_witness(program, witness)?;
    }
    Ok(())
}
