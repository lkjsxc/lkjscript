use std::collections::HashSet;

use crate::verify::*;
use crate::{
    MemoryWitnessId, Program, StructuralValueCategory, MAX_MEMORY_WITNESSES,
    MAX_MEMORY_WITNESS_DEPENDENCIES,
};

pub(super) fn verify(program: &Program) -> crate::Result<()> {
    let records = &program.memory.witnesses;
    if records.len() > MAX_MEMORY_WITNESSES {
        return fail("SSA memory witness table exceeds bounded maximum");
    }
    let mut prior = None;
    let mut types = HashSet::new();
    let mut edges = 0usize;
    for record in records {
        if !record.id.is_resolved() || prior.is_some_and(|id| id >= record.id) {
            return fail("SSA memory witness table must be sorted with unique nonzero IDs");
        }
        prior = Some(record.id);
        if !types.insert(record.ty.clone()) {
            return fail("SSA memory witness table has duplicate semantic types");
        }
        verify_type(program, &record.ty, &[])?;
        validate_facts(record)?;
        let semantic_contract = lkjscript_contracts::semantic_contract_hash(&record.facts.semantic)
            .map_err(|error| crate::IrError::new(error.to_string()))?;
        let semantic_type = lkjscript_contracts::semantic_type_closure_hash(&record.facts.semantic)
            .map_err(|error| crate::IrError::new(error.to_string()))?;
        if semantic_contract != record.facts.semantic_contract
            || semantic_type != record.facts.semantic_type
        {
            return fail("SSA memory witness semantic identity is noncanonical");
        }
        lkjscript_contracts::validate_executable_dependencies(
            &record.facts.semantic,
            &record.dependencies,
        )
        .map_err(|error| crate::IrError::new(error.to_string()))?;
        let recomputed =
            MemoryWitnessId::new(lkjscript_contracts::executable_memory_witness_member_id(
                record.group.bytes(),
                record.ordinal,
                semantic_type,
            ));
        if recomputed != record.id {
            return fail("SSA executable memory witness member identity is noncanonical");
        }
        validate_representation(program, record)?;
        edges = edges
            .checked_add(record.dependencies.len())
            .ok_or_else(|| crate::IrError::new("SSA witness edge work overflow"))?;
        if edges > MAX_MEMORY_WITNESS_DEPENDENCIES {
            return fail("SSA memory witness dependency table exceeds bounded maximum");
        }
    }
    Ok(())
}

fn validate_facts(record: &crate::MemoryWitnessDescriptor) -> crate::Result<()> {
    if record.facts.semantic_type == [0; 32]
        || record.facts.semantic_contract == [0; 32]
        || record.facts.alignment == 0
        || !record.facts.alignment.is_power_of_two()
        || record.facts.alignment > 4_096
        || record.facts.operations.is_empty()
        || record
            .facts
            .operations
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return fail("SSA memory witness descriptor has invalid closed facts");
    }
    if !lkjscript_contracts::memory_witness_routes_are_compatible(&record.facts) {
        return fail("SSA memory witness capability and operation routes are incompatible");
    }
    Ok(())
}

fn validate_representation(
    program: &Program,
    record: &crate::MemoryWitnessDescriptor,
) -> crate::Result<()> {
    let Some(representation) = record.representation else {
        return Ok(());
    };
    let exact = program
        .memory
        .representations
        .get(representation.index().unwrap_or(usize::MAX))
        .filter(|item| item.id == representation && item.category == StructuralValueCategory::Owner)
        .and_then(|item| {
            program
                .memory
                .types
                .get(item.type_id.index().unwrap_or(usize::MAX))
        })
        .ok_or_else(|| {
            crate::IrError::new("SSA memory witness has a stale executable representation route")
        })?;
    let expected_mode = match exact.mode {
        crate::StructuralTypeMode::Copy => lkjscript_contracts::MemoryWitnessMode::Copy,
        crate::StructuralTypeMode::Immutable => {
            lkjscript_contracts::MemoryWitnessMode::ImmutableValue
        }
        crate::StructuralTypeMode::Affine => lkjscript_contracts::MemoryWitnessMode::Affine,
    };
    if exact.ty != record.ty
        || exact.witness != record.id
        || record.facts.mode != expected_mode
        || record.facts.root != lkjscript_contracts::MemoryWitnessRoot::Structural
    {
        return fail("SSA memory witness structural type relinking failed");
    }
    Ok(())
}
