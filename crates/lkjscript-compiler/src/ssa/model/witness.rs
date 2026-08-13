fn install_memory_witnesses(
    memory: &mut StructuralMemoryMetadata,
    plan: &HirMemoryPlan,
    products: &HashMap<crate::hir::ProductId, ProductId>,
) -> Result<()> {
    let installed: std::collections::HashSet<_> = plan.witnesses.iter()
        .filter(|witness| witness_is_installable(witness))
        .map(|witness| witness.id).collect();
    memory.witness_groups = plan.witness_groups.iter()
        .filter(|group| group.members.iter().all(|member| installed.contains(&member.witness)))
        .map(|group| lkjscript_ir::MemoryWitnessGroupDescriptor {
            id: lkjscript_ir::MemoryWitnessGroupId::new(group.id.as_bytes()),
            recursive: group.recursive,
            members: group.members.iter().map(|member|
                lkjscript_ir::MemoryWitnessGroupMember {
                    witness: MemoryWitnessId::new(member.witness.as_bytes()),
                    ordinal: member.ordinal,
                    semantic_identity: member.semantic_identity,
                }).collect(),
        }).collect();
    for witness in &plan.witnesses {
        let facts = &witness.facts;
        if !witness_is_installable(witness) { continue; }
        let group = witness
            .group
            .ok_or_else(|| Error::msg("installable memory witness has no group"))?;
        let ordinal = witness
            .ordinal
            .ok_or_else(|| Error::msg("installable memory witness has no ordinal"))?;
        let ty = lower_memory_type(&facts.ty, products)?;
        let route = fallback_route(
            witness.id,
            crate::memory_plan::MemoryValueCategory::Owner,
            crate::memory_plan::MemoryDomain::UniqueStructural,
            2,
        )?;
        let representation = memory.type_for(&ty).and_then(|item| {
            memory.representation_by_route(
                &item.ty,
                route,
                StructuralValueCategory::Owner,
                StructuralStorage::UniqueStructural,
            )
        });
        let dependencies = facts.dependencies.clone();
        memory.witnesses.push(lkjscript_ir::MemoryWitnessDescriptor {
            id: MemoryWitnessId::new(witness.id.as_bytes()),
            group: lkjscript_ir::MemoryWitnessGroupId::new(group.as_bytes()),
            ordinal,
            facts: crate::memory_plan::executable_facts(facts)?,
            ty,
            dependencies,
            representation,
        });
    }
    memory.witnesses.sort_by_key(|item| item.id);
    Ok(())
}

fn witness_is_installable(witness: &crate::memory_plan::MemoryWitness) -> bool {
    !matches!(witness.facts.requirement,
        crate::memory_plan::MemoryWitnessRequirement::SpecializationRequired)
        && !matches!(witness.facts.ty, MemoryType::Never | MemoryType::ForAll { .. })
}
