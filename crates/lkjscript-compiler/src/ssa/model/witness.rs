use lkjscript_contracts::{
    ExecutableMemoryWitnessFacts, MemoryWitnessCodec, MemoryWitnessContention,
    MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality,
    MemoryWitnessListElement, MemoryWitnessMode, MemoryWitnessOperation,
    MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize,
};

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
            facts: executable_facts(facts)?,
            ty,
            dependencies,
            representation,
        });
    }
    memory.witnesses.sort_by_key(|item| item.id);
    Ok(())
}

fn executable_facts(
    facts: &crate::memory_plan::MemoryWitnessFacts,
) -> Result<ExecutableMemoryWitnessFacts> {
    let operations = witness_operations(facts);
    Ok(ExecutableMemoryWitnessFacts {
        semantic_type: crate::memory_plan::memory_type_identity(&facts.ty)?,
        semantic_contract: facts.semantic_contract,
        mode: map_mode(facts.mode),
        domain: map_domain(facts.domain),
        root: match facts.root_projection {
            crate::memory_plan::MemoryRootProjection::None => MemoryWitnessRoot::None,
            crate::memory_plan::MemoryRootProjection::Structural => MemoryWitnessRoot::Structural,
        },
        copy: map_copy(facts.copy_share),
        drop: witness_drop(facts),
        equality: map_equality(facts.equality),
        codec: map_codec(facts.process_codec),
        list_element: map_list_element(facts.list_element),
        size: witness_size(&facts.ty, facts.dynamic_size),
        alignment: witness_alignment(&facts.ty),
        contains_borrow: facts.contains_borrow,
        contains_dynamic_owner: facts.contains_dynamic_owner,
        portability: map_portability(facts.portability),
        contention: map_contention(facts.contention),
        operations,
    })
}

fn witness_operations(
    facts: &crate::memory_plan::MemoryWitnessFacts,
) -> Vec<MemoryWitnessOperation> {
    let mut operations = vec![MemoryWitnessOperation::Transport];
    if !matches!(facts.copy_share, crate::memory_plan::MemoryCopySharePlan::Unsupported) {
        operations.push(MemoryWitnessOperation::Clone);
    }
    if facts.drop_glue.is_some() || matches!(facts.domain, crate::memory_plan::MemoryDomain::OrdinaryRegion) {
        operations.push(MemoryWitnessOperation::Drop);
    }
    if matches!(facts.copy_share, crate::memory_plan::MemoryCopySharePlan::SealedShare) {
        operations.push(MemoryWitnessOperation::Share);
    }
    if !matches!(facts.equality, crate::memory_plan::MemoryEqualitySupport::Unsupported) {
        operations.push(MemoryWitnessOperation::Compare);
    }
    if matches!(facts.process_codec, crate::memory_plan::MemoryProcessCodecEligibility::Eligible) {
        operations.extend([MemoryWitnessOperation::Encode, MemoryWitnessOperation::Decode]);
    }
    if matches!(facts.list_element, crate::memory_plan::MemoryListElementEligibility::Copy | crate::memory_plan::MemoryListElementEligibility::ImmutableValue) {
        operations.extend([MemoryWitnessOperation::ListImport, MemoryWitnessOperation::ListExport]);
    }
    operations.sort_unstable();
    operations.dedup();
    operations
}

include!("witness_mappings.rs");
