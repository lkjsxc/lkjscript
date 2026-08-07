fn install_authenticated_return_witness(
    chunk: &mut Chunk,
    mode: crate::StructuralTypeMode,
) {
    use lkjscript_contracts::{
        ExecutableMemoryWitnessFacts, MemoryWitnessCapabilities, MemoryWitnessCodec,
        MemoryWitnessContention, MemoryWitnessCopy, MemoryWitnessDomain, MemoryWitnessDrop,
        MemoryWitnessEquality, MemoryWitnessListElement, MemoryWitnessMode,
        MemoryWitnessPortability, MemoryWitnessRoot, MemoryWitnessSize, SemanticDescriptor,
        SemanticPrimitiveKind, SemanticType,
    };

    let immutable = mode == crate::StructuralTypeMode::Immutable;
    let semantic = SemanticDescriptor {
        root: SemanticType::Primitive(SemanticPrimitiveKind::Unit),
        declarations: Vec::new(),
    };
    let mut facts = ExecutableMemoryWitnessFacts {
        semantic_type: lkjscript_contracts::semantic_type_closure_hash(&semantic)
            .expect("authenticated return semantic type closure"),
        semantic_contract: lkjscript_contracts::semantic_contract_hash(&semantic)
            .expect("authenticated return semantic contract"),
        semantic,
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
    let semantic_identity = facts.semantic_type;
    let member = lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
        id: [0; 32], ordinal: 0, semantic_identity,
        facts: facts.clone(), dependencies: Vec::new(),
    };
    let group = lkjscript_contracts::executable_memory_witness_group_id(false, &[member]);
    let group = crate::MemoryWitnessGroupId::new(group);
    let id = crate::MemoryWitnessId::new(
        lkjscript_contracts::executable_memory_witness_member_id(
            group.bytes(), 0, semantic_identity));
    chunk.structural_types[0].witness = id;
    for representation in &mut chunk.structural_representations {
        representation.witness = id;
        representation.witness_group = group;
        representation.witness_member = 0;
    }
    chunk.memory_witnesses = vec![crate::InstalledMemoryWitness {
        id, group, ordinal: 0, facts, dependencies: Vec::new(),
        value_kind: crate::MemoryWitnessValueKind::Structural(
            crate::StructuralRepresentationId::new(0)),
    }];
    chunk.memory_witness_groups = vec![crate::InstalledMemoryWitnessGroup {
        id: group, recursive: false,
        members: vec![crate::InstalledMemoryWitnessGroupMember {
            witness: id, ordinal: 0, semantic_identity,
        }],
    }];
}

#[test]
fn authenticated_return_uses_one_owner_count_for_the_whole_dag() {
    let chunk = returning_product_chunk();
    let field_type = copy_field().runtime_type.expect("field type");
    let snapshot = returned_product_snapshot(&chunk, field_type, 1);
    assert_eq!(snapshot.nodes().len(), 2);
    let mut runtime = crate::SealedSemanticDagRuntime::new().expect("sealed runtime");
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
