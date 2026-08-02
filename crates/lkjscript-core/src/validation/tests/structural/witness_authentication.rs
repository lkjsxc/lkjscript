use lkjscript_contracts::{
    ExecutableMemoryWitnessFacts, MemoryWitnessCapabilities, MemoryWitnessCodec,
    MemoryWitnessContention,
    MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality,
    MemoryWitnessListElement, MemoryWitnessMode, MemoryWitnessOperation,
    MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize,
};

fn scalar_witness(
    facts: ExecutableMemoryWitnessFacts,
    dependencies: Vec<crate::MemoryWitnessId>,
) -> crate::InstalledMemoryWitness {
    let dependency_bytes: Vec<_> = dependencies.iter().map(|item| item.bytes()).collect();
    let encoded = lkjscript_contracts::canonical_executable_memory_witness(
        &facts,
        &dependency_bytes,
    );
    crate::InstalledMemoryWitness {
        id: crate::MemoryWitnessId::new(crate::sha256(&encoded)),
        facts,
        dependencies,
        value_kind: crate::MemoryWitnessValueKind::I64,
    }
}

fn scalar_facts() -> ExecutableMemoryWitnessFacts {
    ExecutableMemoryWitnessFacts {
        semantic_type: [1; 32],
        semantic_contract: [2; 32],
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
        ],
    }
}

fn witness_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    let plan = crate::MemoryPlanId::new([8; 32]);
    chunk.memory_plan = Some(plan);
    chunk.main.memory_plan = Some(plan);
    chunk.memory_witnesses = vec![scalar_witness(scalar_facts(), Vec::new())];
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn bytecode_recomputes_executable_witness_identity() {
    let chunk = witness_chunk();
    validate_chunk(chunk.clone(), &ValidationLimits::default())
        .expect("canonical executable witness validates");

    let mut changed = chunk.clone();
    changed.memory_witnesses[0].facts.semantic_contract = [3; 32];
    assert!(error(changed).contains("identity is noncanonical"));

    let mut changed = chunk.clone();
    changed.memory_witnesses[0].facts.copy = MemoryWitnessCopy::SealedShare;
    assert!(error(changed).contains("capability and operation routes are incompatible"));

    let mut changed = chunk.clone();
    changed.memory_witnesses[0]
        .facts
        .operations
        .insert(2, MemoryWitnessOperation::Share);
    assert!(error(changed).contains("operation routes are incompatible"));

    let mut recomputed = chunk;
    let mut facts = scalar_facts();
    facts.operations.insert(2, MemoryWitnessOperation::Share);
    recomputed.memory_witnesses[0] = scalar_witness(facts, Vec::new());
    assert!(error(recomputed).contains("operation routes are incompatible"));
}

#[test]
fn bytecode_witness_dependency_changes_are_identity_bearing() {
    let mut changed = witness_chunk();
    changed.memory_witnesses[0]
        .dependencies
        .push(crate::MemoryWitnessId::new([9; 32]));
    assert!(error(changed).contains("identity is noncanonical"));

    let mut missing = witness_chunk();
    let dependency = crate::MemoryWitnessId::new([9; 32]);
    missing.memory_witnesses[0] = scalar_witness(scalar_facts(), vec![dependency]);
    assert!(error(missing).contains("dependency is missing"));
}
