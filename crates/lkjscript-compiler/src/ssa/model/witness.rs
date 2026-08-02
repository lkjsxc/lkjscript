fn install_memory_witnesses(
    memory: &mut StructuralMemoryMetadata,
    plan: &HirMemoryPlan,
    products: &HashMap<String, ProductId>,
) -> Result<()> {
    for witness in &plan.witnesses {
        let facts = &witness.facts;
        if matches!(facts.requirement, crate::memory_plan::MemoryWitnessRequirement::SpecializationRequired)
            || matches!(facts.ty, MemoryType::Never | MemoryType::ForAll { .. })
        {
            continue;
        }
        let ty = lower_memory_type(&facts.ty, products)?;
        let representation = memory
            .type_for(&ty)
            .and_then(|item| memory.representation(&item.ty, StructuralValueCategory::Owner));
        let dependencies = facts
            .list
            .iter()
            .map(|list| MemoryWitnessId::new(list.element.as_bytes()))
            .collect();
        memory.witnesses.push(lkjscript_ir::MemoryWitnessDescriptor {
            id: MemoryWitnessId::new(witness.id.as_bytes()),
            facts: crate::memory_plan::executable_facts(facts)?,
            ty,
            dependencies,
            representation,
        });
    }
    memory.witnesses.sort_by_key(|item| item.id);
    Ok(())
}
