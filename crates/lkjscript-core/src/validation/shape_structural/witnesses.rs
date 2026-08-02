use std::collections::{BTreeSet, HashSet};

fn validate_witnesses(chunk: &Chunk) -> Result<usize> {
    let records = &chunk.memory_witnesses;
    let mut prior = None;
    let mut edges = 0usize;
    let mut bytes = 0usize;
    let mut incoming = vec![0usize; records.len()];
    let mut outgoing = vec![Vec::new(); records.len()];
    let mut representations = HashSet::new();
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
        if !lkjscript_contracts::memory_witness_routes_are_compatible(&record.facts) {
            return Err(Error::msg(
                "bytecode memory witness capability and operation routes are incompatible",
            ));
        }
        let semantic_contract = lkjscript_contracts::semantic_contract_hash(&record.facts.semantic)
            .map_err(|error| Error::msg(error.to_string()))?;
        if semantic_contract != record.facts.semantic_contract {
            return Err(Error::msg("bytecode memory witness semantic contract is noncanonical"));
        }
        let semantic_type =
            lkjscript_contracts::semantic_type_closure_hash(&record.facts.semantic)
                .map_err(|error| Error::msg(error.to_string()))?;
        if semantic_type != record.facts.semantic_type {
            return Err(Error::msg(
                "bytecode memory witness semantic type closure is noncanonical",
            ));
        }
        lkjscript_contracts::validate_executable_dependencies(
            &record.facts.semantic,
            &record.dependencies,
        )
        .map_err(|error| Error::msg(error.to_string()))?;
        let category = match record.value_kind {
            crate::MemoryWitnessValueKind::Unit=>0, crate::MemoryWitnessValueKind::Bool=>1,
            crate::MemoryWitnessValueKind::I64=>2, crate::MemoryWitnessValueKind::F64=>3,
            crate::MemoryWitnessValueKind::List=>4, crate::MemoryWitnessValueKind::Structural(_)=>5,
            crate::MemoryWitnessValueKind::Unsupported=>6,
        };
        if !representations.insert((record.facts.semantic_type, category)) {
            return Err(Error::msg("duplicate bytecode semantic type and owner representation"));
        }
        let encoded = lkjscript_contracts::canonical_executable_memory_witness(
            &record.facts,
            &record.dependencies,
        );
        if crate::MemoryWitnessId::new(crate::sha256(&encoded)) != record.id {
            return Err(Error::msg(
                "bytecode executable memory witness identity is noncanonical",
            ));
        }
        validate_witness_route(chunk, record)?;
        let requirements = lkjscript_contracts::semantic_dependency_requirements(
            &record.facts.semantic,
        )
        .map_err(|error| Error::msg(error.to_string()))?;
        let mut roles = HashSet::new();
        for (dependency, (_, expected_ty)) in record.dependencies.iter().zip(requirements) {
            if !roles.insert(dependency.role.clone()) {
                return Err(Error::msg("duplicate bytecode complete witness dependency role"));
            }
            edges = add(edges, 1, "memory witness dependency work")?;
            if edges > crate::MAX_MEMORY_WITNESS_DEPENDENCIES {
                return Err(Error::msg("memory witness dependency limit exceeded"));
            }
            if let lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalWitness(bytes) = dependency.target {
                let child = witness_index(records, crate::MemoryWitnessId::new(bytes))?;
                if records[child].facts.semantic.root != expected_ty {
                    return Err(Error::msg("bytecode external witness dependency has wrong semantic type"));
                }
                outgoing[index].push(child);
                incoming[child] = add(incoming[child], 1, "memory witness indegree")?;
            }
        }
        bytes = add(
            bytes,
            128usize
                .saturating_add(record.dependencies.len().saturating_mul(160))
                .saturating_add(
                    lkjscript_contracts::canonical_semantic_descriptor(&record.facts.semantic)
                        .map_err(|error| Error::msg(error.to_string()))?
                        .len(),
                )
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
        .and_then(|item| chunk.structural_types.get(item.type_id.index()))
        .ok_or_else(|| Error::msg("bytecode memory witness structural route is stale"))?;
    let expected_mode = match ty.mode {
        crate::StructuralTypeMode::Copy => lkjscript_contracts::MemoryWitnessMode::Copy,
        crate::StructuralTypeMode::Immutable => {
            lkjscript_contracts::MemoryWitnessMode::ImmutableValue
        }
        crate::StructuralTypeMode::Affine => lkjscript_contracts::MemoryWitnessMode::Affine,
    };
    if ty.witness != witness.id
        || witness.facts.mode != expected_mode
        || witness.facts.root != lkjscript_contracts::MemoryWitnessRoot::Structural
    {
        return Err(Error::msg(
            "bytecode memory witness structural type relinking failed",
        ));
    }
    Ok(())
}

fn witness_index(records: &[crate::InstalledMemoryWitness], id: crate::MemoryWitnessId) -> Result<usize> {
    records
        .binary_search_by_key(&id, |record| record.id)
        .map_err(|_| Error::msg("bytecode memory witness dependency is missing"))
}
