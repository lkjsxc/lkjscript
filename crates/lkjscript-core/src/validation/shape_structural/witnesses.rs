use std::collections::{BTreeSet, HashSet};

fn validate_witnesses(chunk: &Chunk) -> Result<usize> {
    let records = &chunk.memory_witnesses;
    let mut prior = None;
    let mut edges = 0usize;
    let mut bytes = 0usize;
    let mut incoming = vec![0usize; records.len()];
    let mut outgoing = vec![Vec::new(); records.len()];
    for (index, record) in records.iter().enumerate() {
        if !record.id.is_resolved() || prior.is_some_and(|id| id >= record.id) {
            return Err(Error::msg(
                "bytecode memory witnesses must have sorted unique nonzero IDs",
            ));
        }
        prior = Some(record.id);
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
            return Err(Error::msg("bytecode memory witness facts are invalid"));
        }
        validate_witness_route(chunk, record)?;
        let mut dependencies = HashSet::new();
        for dependency in &record.dependencies {
            if !dependencies.insert(*dependency) {
                return Err(Error::msg("duplicate bytecode memory witness dependency"));
            }
            edges = add(edges, 1, "memory witness dependency work")?;
            if edges > crate::MAX_MEMORY_WITNESS_DEPENDENCIES {
                return Err(Error::msg("memory witness dependency limit exceeded"));
            }
            let child = witness_index(records, *dependency)?;
            outgoing[index].push(child);
            incoming[child] = add(incoming[child], 1, "memory witness indegree")?;
        }
        bytes = add(
            bytes,
            128usize
                .saturating_add(record.dependencies.len().saturating_mul(32))
                .saturating_add(record.facts.operations.len()),
            "memory witness metadata bytes",
        )?;
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
        return Err(Error::msg("bytecode memory witness dependency graph is cyclic"));
    }
    Ok(bytes)
}

fn validate_witness_route(chunk: &Chunk, witness: &crate::InstalledMemoryWitness) -> Result<()> {
    let crate::MemoryWitnessValueKind::Structural(representation) = witness.value_kind else {
        return Ok(());
    };
    let ty = chunk
        .structural_representations
        .get(representation.index())
        .filter(|item| {
            item.id == representation && item.category == StructuralValueCategory::Owner
        })
        .and_then(|item| chunk.structural_types.get(item.type_id.index()));
    if ty.is_none_or(|item| item.witness != witness.id) {
        return Err(Error::msg(
            "bytecode memory witness structural route is stale",
        ));
    }
    Ok(())
}

fn witness_index(records: &[crate::InstalledMemoryWitness], id: crate::MemoryWitnessId) -> Result<usize> {
    records
        .binary_search_by_key(&id, |record| record.id)
        .map_err(|_| Error::msg("bytecode memory witness dependency is missing"))
}
