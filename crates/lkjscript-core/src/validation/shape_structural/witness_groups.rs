fn validate_witness_groups(chunk: &Chunk) -> Result<usize> {
    let groups = &chunk.memory_witness_groups;
    let mut prior = None;
    let mut covered = HashSet::new();
    let contracts = groups.iter().map(|group| {
        if !group.id.is_resolved() || prior.is_some_and(|id| id >= group.id) {
            return Err(Error::msg(
                "bytecode memory witness groups require sorted unique nonzero IDs"));
        }
        prior = Some(group.id);
        let members = group.members.iter().map(|member| {
            let witness = chunk.memory_witnesses.binary_search_by_key(
                &member.witness, |item| item.id).ok()
                .and_then(|index| chunk.memory_witnesses.get(index))
                .ok_or_else(|| Error::msg("bytecode memory witness group member is missing"))?;
            if !covered.insert(member.witness) || witness.group != group.id
                || witness.ordinal != member.ordinal
                || witness.facts.semantic_type != member.semantic_identity {
                return Err(Error::msg(
                    "bytecode memory witness group partition is inconsistent"));
            }
            Ok(lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
                id: witness.id.bytes(), ordinal: member.ordinal,
                semantic_identity: member.semantic_identity, facts: witness.facts.clone(),
                dependencies: witness.dependencies.clone(),
            })
        }).collect::<Result<Vec<_>>>()?;
        Ok(lkjscript_contracts::ExecutableMemoryWitnessGroup {
            id: group.id.bytes(), recursive: group.recursive, members,
        })
    }).collect::<Result<Vec<_>>>()?;
    if covered.len() != chunk.memory_witnesses.len() {
        return Err(Error::msg(
            "bytecode memory witness groups have missing or extra members"));
    }
    lkjscript_contracts::validate_executable_memory_witness_groups(&contracts)
        .map_err(|error| Error::msg(error.to_string()))?;
    groups.iter().try_fold(0usize, |bytes, group| {
        let member_bytes = group
            .members
            .len()
            .checked_mul(64)
            .ok_or_else(|| Error::host("bytecode memory witness group byte size overflow"))?;
        let bytes = add(bytes, 64, "memory witness group byte size")?;
        add(bytes, member_bytes, "memory witness group byte size")
    })
}
