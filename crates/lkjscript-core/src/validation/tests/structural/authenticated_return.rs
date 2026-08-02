fn install_authenticated_return_witness(
    chunk: &mut Chunk,
    mode: crate::StructuralTypeMode,
) {
    use lkjscript_contracts::{
        ExecutableMemoryWitnessFacts, MemoryWitnessCapabilities, MemoryWitnessCodec,
        MemoryWitnessContention, MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop,
        MemoryWitnessEquality, MemoryWitnessListElement, MemoryWitnessMode,
        MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize,
    };

    let immutable = mode == crate::StructuralTypeMode::Immutable;
    let mut facts = ExecutableMemoryWitnessFacts {
        semantic_type: [11; 32],
        semantic_contract: [12; 32],
        mode: match mode {
            crate::StructuralTypeMode::Copy => MemoryWitnessMode::Copy,
            crate::StructuralTypeMode::Immutable => MemoryWitnessMode::ImmutableValue,
            crate::StructuralTypeMode::Affine => MemoryWitnessMode::Affine,
        },
        capabilities: MemoryWitnessCapabilities {
            inline: false,
            static_value: false,
            unique: true,
            ordinary_region: immutable,
            sealed_region: immutable,
            borrow: true,
            process_codec: immutable,
            list_element: immutable,
            equality: false,
        },
        domain: MemoryWitnessDomain::UniqueStructural,
        root: MemoryWitnessRoot::Structural,
        copy: MemoryWitnessCopy::Structural,
        drop: MemoryWitnessDrop::Structural,
        equality: MemoryWitnessEquality::Unsupported,
        codec: if immutable {
            MemoryWitnessCodec::Eligible
        } else {
            MemoryWitnessCodec::Ineligible
        },
        list_element: if immutable {
            MemoryWitnessListElement::ImmutableValue
        } else {
            MemoryWitnessListElement::UnsupportedAffine
        },
        size: MemoryWitnessSize::CheckedDynamic,
        alignment: 8,
        contains_borrow: false,
        contains_dynamic_owner: false,
        portability: MemoryWitnessPortability::WorkerLocal,
        contention: MemoryWitnessContention::SingleOwner,
        operations: Vec::new(),
    };
    facts.operations = lkjscript_contracts::required_memory_witness_operations(&facts);
    let encoded = lkjscript_contracts::canonical_executable_memory_witness(&facts, &[]);
    let id = crate::MemoryWitnessId::new(crate::sha256(&encoded));
    chunk.structural_types[0].witness = id;
    chunk.memory_witnesses = vec![crate::InstalledMemoryWitness {
        id,
        facts,
        dependencies: Vec::new(),
        value_kind: crate::MemoryWitnessValueKind::Structural(
            crate::StructuralRepresentationId::new(0),
        ),
    }];
}

#[test]
fn authenticated_return_uses_one_owner_count_for_the_whole_dag() {
    let chunk = returning_product_chunk();
    let field_type = copy_field().runtime_type.expect("field type");
    let snapshot = returned_product_snapshot(&chunk, field_type, 1);
    assert_eq!(snapshot.nodes().len(), 2);
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect("rehydrate two-node DAG");
    let retained = runtime.retain(&owner).expect("retain DAG owner");
    let metrics = runtime.metrics();
    assert_eq!(metrics.runtime.domains_created, 1);
    assert_eq!(metrics.sealed.regions_sealed, 1);
    assert_eq!(metrics.sealed.retains, 1);
    assert_eq!(
        runtime
            .release(retained)
            .expect("release retained owner")
            .regions_released,
        0
    );
    assert_eq!(
        runtime
            .release(owner)
            .expect("release final owner")
            .regions_released,
        1
    );
    assert_eq!(runtime.metrics().sealed.release_work, 2);
}
