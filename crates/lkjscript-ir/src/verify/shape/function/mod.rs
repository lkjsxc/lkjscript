use std::collections::HashSet;

use crate::verify::*;
use crate::{BlockId, BorrowKind, FailureCleanupAction, Function, Program, SsaType, TraitRole};

pub(crate) fn verify_function(program: &Program, function: &Function) -> crate::Result<()> {
    precheck_ownership_work_shape(function)?;
    let type_parameters: Vec<&str> = function
        .signature
        .type_parameters
        .iter()
        .map(String::as_str)
        .collect();
    let mut seen_type_parameters = HashSet::new();
    if type_parameters
        .iter()
        .any(|name| name.is_empty() || !seen_type_parameters.insert(*name))
    {
        return fail(format!(
            "SSA function {} has invalid type parameters",
            function.name
        ));
    }
    verify_witness_parameters(&function.signature, &type_parameters)?;
    let mut seen_bounds = HashSet::new();
    for bound in &function.signature.bounds {
        if !type_parameters.contains(&bound.parameter.as_str()) {
            return fail(format!(
                "SSA function {} has a bound on undeclared parameter {}",
                function.name, bound.parameter
            ));
        }
        let trait_metadata = trait_by_id(program, bound.trait_id)?;
        if matches!(trait_metadata.role, TraitRole::Clone | TraitRole::Drop) {
            return fail(format!(
                "SSA function {} uses a core trait that requires unavailable methods",
                function.name
            ));
        }
        if !seen_bounds.insert((bound.parameter.as_str(), bound.trait_id)) {
            return fail(format!(
                "SSA function {} has duplicate trait bounds",
                function.name
            ));
        }
    }
    for ty in &function.signature.parameters {
        verify_type(program, ty, &type_parameters)?;
    }
    let mut place_bindings = HashSet::new();
    for (index, place) in function.places.iter().enumerate() {
        if place.id.index() != Some(index) || !place_bindings.insert(place.binding) {
            return fail("SSA places must have dense IDs and unique binding identities");
        }
        verify_type(program, &place.ty, &type_parameters)?;
        let expected_glue = expected_drop_glue(program, &place.ty);
        if place.drop_glue.is_some() && place.drop_glue != expected_glue {
            return fail(format!(
                "SSA place {} in {} has mismatched drop glue: actual {:?}, expected {:?}, type {:?}",
                place.id.raw(), function.name, place.drop_glue, expected_glue, place.ty
            ));
        }
        if is_byte_vector(&place.ty) && place.drop_glue.is_none() {
            return fail("SSA byte-vector place is missing its drop-glue obligation");
        }
    }
    verify_type(program, &function.signature.result, &type_parameters)?;
    if matches!(
        function.signature.result.as_ref(),
        SsaType::ByteSlice | SsaType::ByteSliceMut
    ) {
        return fail("SSA function cannot return a lexical reference in this slice");
    }

    if function.blocks.is_empty() {
        return fail(format!("SSA function {} has no blocks", function.name));
    }
    if function.blocks.len() > SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION {
        return fail(format!(
            "SSA function {} exceeds {SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION} blocks",
            function.name
        ));
    }
    if function
        .entry
        .index()
        .is_none_or(|index| index >= function.blocks.len())
    {
        return fail(format!(
            "SSA function {} has an invalid entry",
            function.name
        ));
    }
    for (index, block) in function.blocks.iter().enumerate() {
        if block.id.index() != Some(index) {
            return fail(format!(
                "SSA function {} must store dense BlockIds in order",
                function.name
            ));
        }
    }

    let (types, definitions) = collect_values(program, function, &type_parameters)?;
    let entry = block(function, function.entry)?;
    if entry.parameters.len() != function.signature.parameters.len() {
        return fail(format!(
            "SSA function {} entry parameter arity does not match signature",
            function.name
        ));
    }
    for (parameter, expected) in entry.parameters.iter().zip(&function.signature.parameters) {
        if &parameter.ty != expected {
            return fail(format!(
                "SSA function {} entry parameter type mismatch",
                function.name
            ));
        }
    }
    verify_failure_cleanup_shape(program, function, &types)?;
    verify_ownership_facts(program, function, &types)?;

    let predecessors = predecessors(function)?;
    let dominators = dominators(function, &predecessors)?;
    let reachable = reachable(function)?;
    for block in &function.blocks {
        verify_block(
            program,
            function,
            block,
            &types,
            &definitions,
            &dominators,
            &type_parameters,
        )?;
    }
    verify_loops(function, &dominators, &reachable)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Definition {
    pub(crate) block: BlockId,
    pub(crate) instruction: Option<usize>,
}

pub(crate) type Dominators = Vec<Vec<u64>>;

include!("failure_cleanup.rs");
