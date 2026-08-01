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
            if exact.is_none_or(|item| item.ty != record.ty || item.witness != record.id) {
                return fail("SSA memory witness has a stale executable representation route");
            }
        }
        let mut dependencies = HashSet::new();
        for dependency in &record.dependencies {
            if !dependencies.insert(*dependency) {
                return fail("SSA memory witness has duplicate dependency edges");
            }
            edges = edges
                .checked_add(1)
                .ok_or_else(|| crate::IrError::new("SSA witness edge work overflow"))?;
            if edges > MAX_MEMORY_WITNESS_DEPENDENCIES {
                return fail("SSA memory witness dependency table exceeds bounded maximum");
            }
            let child = witness_index(records, *dependency)?;
            outgoing[index].push(child);
            incoming[child] = incoming[child]
                .checked_add(1)
                .ok_or_else(|| crate::IrError::new("SSA witness indegree overflow"))?;
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
