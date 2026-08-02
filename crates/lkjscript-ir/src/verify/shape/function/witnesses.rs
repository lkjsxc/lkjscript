use std::collections::{BTreeSet, HashSet};

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
    let mut incoming = vec![0usize; records.len()];
    let mut outgoing = vec![Vec::new(); records.len()];
    for (index, record) in records.iter().enumerate() {
        if !record.id.is_resolved() || prior.is_some_and(|id| id >= record.id) {
            return fail("SSA memory witness table must be sorted with unique nonzero IDs");
        }
        prior = Some(record.id);
        if !types.insert(record.ty.clone()) {
            return fail("SSA memory witness table has duplicate semantic types");
        }
        verify_type(program, &record.ty, &[])?;
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
        let semantic_contract = lkjscript_contracts::semantic_contract_hash(&record.facts.semantic)
            .map_err(|error| crate::IrError::new(error.to_string()))?;
        if semantic_contract != record.facts.semantic_contract {
            return fail("SSA memory witness semantic contract is noncanonical");
        }
        let semantic_type = lkjscript_contracts::semantic_type_closure_hash(&record.facts.semantic)
            .map_err(|error| crate::IrError::new(error.to_string()))?;
        if semantic_type != record.facts.semantic_type {
            return fail("SSA memory witness semantic type closure is noncanonical");
        }
        lkjscript_contracts::validate_executable_dependencies(
            &record.facts.semantic,
            &record.dependencies,
        )
        .map_err(|error| crate::IrError::new(error.to_string()))?;
        let encoded = lkjscript_contracts::canonical_executable_memory_witness(
            &record.facts,
            &record.dependencies,
        );
        let recomputed = MemoryWitnessId::new(lkjscript_core::sha256(&encoded));
        if recomputed != record.id {
            return fail("SSA executable memory witness identity is noncanonical");
        }
        if let Some(representation) = record.representation {
            let exact = program
                .memory
                .representations
                .get(representation.index().unwrap_or(usize::MAX))
                .filter(|item| {
                    item.id == representation && item.category == StructuralValueCategory::Owner
                })
                .and_then(|item| {
                    program
                        .memory
                        .types
                        .get(item.type_id.index().unwrap_or(usize::MAX))
                });
            let exact = exact.ok_or_else(|| {
                crate::IrError::new(
                    "SSA memory witness has a stale executable representation route",
                )
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
        }
        let requirements =
            lkjscript_contracts::semantic_dependency_requirements(&record.facts.semantic)
                .map_err(|error| crate::IrError::new(error.to_string()))?;
        let mut roles = HashSet::new();
        for (dependency, (_, expected_ty)) in record.dependencies.iter().zip(requirements) {
            if !roles.insert(dependency.role.clone()) {
                return fail("SSA memory witness has duplicate complete dependency roles");
            }
            edges = edges
                .checked_add(1)
                .ok_or_else(|| crate::IrError::new("SSA witness edge work overflow"))?;
            if edges > MAX_MEMORY_WITNESS_DEPENDENCIES {
                return fail("SSA memory witness dependency table exceeds bounded maximum");
            }
            if let lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalWitness(bytes) =
                dependency.target
            {
                let child = witness_index(records, MemoryWitnessId::new(bytes))?;
                if records[child].facts.semantic.root != expected_ty {
                    return fail("SSA external witness dependency has the wrong semantic type");
                }
                outgoing[index].push(child);
                incoming[child] = incoming[child]
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA witness indegree overflow"))?;
            }
        }
    }
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect();
    let mut visited = 0usize;
    while let Some(index) = ready.pop_first() {
        visited += 1;
        for child in &outgoing[index] {
            incoming[*child] -= 1;
            if incoming[*child] == 0 {
                ready.insert(*child);
            }
        }
    }
    if visited != records.len() {
        return fail("SSA executable memory witness dependency graph is cyclic");
    }
    Ok(())
}

fn witness_index(
    records: &[crate::MemoryWitnessDescriptor],
    id: MemoryWitnessId,
) -> crate::Result<usize> {
    records
        .binary_search_by_key(&id, |record| record.id)
        .map_err(|_| crate::IrError::new("SSA memory witness dependency is missing"))
}
