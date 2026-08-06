use lkjscript_contracts::{
    ExecutableMemoryWitnessDependency, ExecutableMemoryWitnessFacts, ExecutableMemoryWitnessRole,
    ExecutableMemoryWitnessTarget, MemoryWitnessCapabilities, MemoryWitnessCodec,
    MemoryWitnessContention, MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop,
    MemoryWitnessEquality, MemoryWitnessListElement, MemoryWitnessMode, MemoryWitnessOperation,
    MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize, SemanticDescriptor,
    SemanticPrimitiveKind, SemanticType,
};

fn scalar_witness(
    mut facts: ExecutableMemoryWitnessFacts,
    dependencies: Vec<ExecutableMemoryWitnessDependency>,
) -> crate::InstalledMemoryWitness {
    facts.semantic_type = lkjscript_contracts::semantic_type_closure_hash(&facts.semantic)
        .expect("scalar semantic type closure");
    let member = lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
        id: [0; 32], ordinal: 0, semantic_identity: facts.semantic_type,
        facts: facts.clone(), dependencies: dependencies.clone(),
    };
    let group = lkjscript_contracts::executable_memory_witness_group_id(false, &[member]);
    crate::InstalledMemoryWitness {
        id: crate::MemoryWitnessId::new(
            lkjscript_contracts::executable_memory_witness_member_id(
                group, 0, facts.semantic_type)),
        group: crate::MemoryWitnessGroupId::new(group),
        ordinal: 0, facts, dependencies,
        value_kind: crate::MemoryWitnessValueKind::I64,
    }
}

fn install_group(chunk: &mut Chunk) {
    chunk.memory_witness_groups = chunk.memory_witnesses.iter().map(|witness|
        crate::InstalledMemoryWitnessGroup {
            id: witness.group, recursive: false,
            members: vec![crate::InstalledMemoryWitnessGroupMember {
                witness: witness.id, ordinal: 0,
                semantic_identity: witness.facts.semantic_type,
            }],
        }).collect();
    chunk.memory_witness_groups.sort_by_key(|group| group.id);
}

fn scalar_facts() -> ExecutableMemoryWitnessFacts {
    let semantic = SemanticDescriptor {
        root: SemanticType::Primitive(SemanticPrimitiveKind::I64),
        declarations: Vec::new(),
    };
    ExecutableMemoryWitnessFacts {
        semantic_type: lkjscript_contracts::semantic_type_closure_hash(&semantic)
            .expect("scalar semantic type closure"),
        semantic_contract: lkjscript_contracts::semantic_contract_hash(&semantic)
            .expect("scalar semantic contract"),
        semantic,
        mode: MemoryWitnessMode::Copy,
        capabilities: MemoryWitnessCapabilities {
            inline: true,
            static_value: false,
            unique: false,
            ordinary_region: false,
            sealed_region: false,
            borrow: true,
            process_codec: true,
            list_element: true,
            equality: true,
        },
        domain: MemoryWitnessDomain::Inline,
        root: MemoryWitnessRoot::None,
        copy: MemoryWitnessCopy::Trivial,
        drop: MemoryWitnessDrop::Trivial,
        equality: MemoryWitnessEquality::Value,
        codec: MemoryWitnessCodec::Eligible,
        list_element: MemoryWitnessListElement::Copy,
        size: MemoryWitnessSize::Fixed(8),
        alignment: 8,
        contains_borrow: false,
        contains_dynamic_owner: false,
        portability: MemoryWitnessPortability::Portable,
        contention: MemoryWitnessContention::None,
        operations: vec![
            MemoryWitnessOperation::Transport,
            MemoryWitnessOperation::Clone,
            MemoryWitnessOperation::Compare,
            MemoryWitnessOperation::Encode,
            MemoryWitnessOperation::Decode,
            MemoryWitnessOperation::ListImport,
            MemoryWitnessOperation::ListExport,
            MemoryWitnessOperation::IndependentOwner,
            MemoryWitnessOperation::Dispose,
        ],
    }
}

fn witness_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    let plan = crate::MemoryPlanId::new([8; 32]);
    chunk.memory_plan = Some(plan);
    chunk.main.memory_plan = Some(plan);
    chunk.memory_witnesses = vec![scalar_witness(scalar_facts(), Vec::new())];
    install_group(&mut chunk);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn bytecode_recomputes_executable_witness_identity() {
    let chunk = witness_chunk();
    validate_chunk(chunk.clone(), ValidationPolicy::Unrestricted)
        .expect("canonical executable witness validates");

    let mut changed = chunk.clone();
    changed.memory_witnesses[0].facts.semantic_contract = [3; 32];
    assert!(error(changed).contains("group identity is noncanonical"));

    let mut changed = chunk.clone();
    changed.memory_witnesses[0].facts.copy = MemoryWitnessCopy::SealedShare;
    assert!(error(changed).contains("group identity is noncanonical"));

    let mut changed = chunk.clone();
    changed.memory_witnesses[0]
        .facts
        .operations
        .insert(2, MemoryWitnessOperation::Share);
    assert!(error(changed).contains("group identity is noncanonical"));

    let mut recomputed = chunk;
    let mut facts = scalar_facts();
    facts.operations.insert(2, MemoryWitnessOperation::Share);
    recomputed.memory_witnesses[0] = scalar_witness(facts, Vec::new());
    install_group(&mut recomputed);
    assert!(error(recomputed).contains("operation routes are incompatible"));
}

#[test]
fn bytecode_witness_dependency_changes_are_identity_bearing() {
    let mut changed = witness_chunk();
    changed.memory_witnesses[0].dependencies.push(ExecutableMemoryWitnessDependency {
        role: ExecutableMemoryWitnessRole::ListElement,
        target: ExecutableMemoryWitnessTarget::ExternalMember {
            group: [8; 32], member: [9; 32],
        },
    });
    assert!(error(changed).contains("dependency closure is invalid"));

    let mut missing = witness_chunk();
    let mut facts = scalar_facts();
    facts.semantic.root = SemanticType::List(Box::new(SemanticType::Primitive(
        SemanticPrimitiveKind::I64,
    )));
    facts.semantic_contract = lkjscript_contracts::semantic_contract_hash(&facts.semantic)
        .expect("list semantic contract");
    let dependency = ExecutableMemoryWitnessDependency {
        role: ExecutableMemoryWitnessRole::ListElement,
        target: ExecutableMemoryWitnessTarget::ExternalMember {
            group: [8; 32], member: [9; 32],
        },
    };
    missing.memory_witnesses[0] = scalar_witness(facts, vec![dependency]);
    install_group(&mut missing);
    assert!(error(missing).contains("external memory witness group is missing"));
}
