use super::*;

fn facts() -> ExecutableMemoryWitnessFacts {
    ExecutableMemoryWitnessFacts {
        semantic_type: [1; 32],
        semantic_contract: [2; 32],
        mode: MemoryWitnessMode::ImmutableValue,
        capabilities: MemoryWitnessCapabilities {
            inline: false,
            static_value: false,
            unique: true,
            ordinary_region: true,
            sealed_region: true,
            borrow: true,
            process_codec: true,
            list_element: true,
            equality: true,
        },
        domain: MemoryWitnessDomain::OrdinaryRegion,
        root: MemoryWitnessRoot::Structural,
        copy: MemoryWitnessCopy::Structural,
        drop: MemoryWitnessDrop::RegionReset,
        equality: MemoryWitnessEquality::Value,
        codec: MemoryWitnessCodec::Eligible,
        list_element: MemoryWitnessListElement::ImmutableValue,
        size: MemoryWitnessSize::CheckedDynamic,
        alignment: 8,
        contains_borrow: false,
        contains_dynamic_owner: true,
        portability: MemoryWitnessPortability::WorkerLocal,
        contention: MemoryWitnessContention::SingleOwner,
        operations: vec![
            MemoryWitnessOperation::Transport,
            MemoryWitnessOperation::Clone,
            MemoryWitnessOperation::Drop,
        ],
    }
}

#[test]
fn executable_witness_encoding_is_deterministic_and_dependency_ordered() {
    let dependencies = [[3; 32], [4; 32]];
    let first = canonical_executable_memory_witness(&facts(), &dependencies);
    let second = canonical_executable_memory_witness(&facts(), &dependencies);
    assert_eq!(first, second);
    assert_ne!(
        first,
        canonical_executable_memory_witness(&facts(), &[[4; 32], [3; 32]])
    );
    assert_ne!(
        first,
        canonical_executable_memory_witness(&facts(), &[[3; 32]])
    );
}

#[test]
fn every_executable_fact_changes_the_canonical_encoding() {
    let baseline = canonical_executable_memory_witness(&facts(), &[[3; 32]]);
    let mut candidates = Vec::new();

    let mut item = facts();
    item.semantic_type = [9; 32];
    candidates.push(item);
    let mut item = facts();
    item.semantic_contract = [9; 32];
    candidates.push(item);
    let mut item = facts();
    item.mode = MemoryWitnessMode::Affine;
    candidates.push(item);
    let capabilities = facts().capabilities;
    for changed in [
        MemoryWitnessCapabilities {
            inline: true,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            static_value: true,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            unique: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            ordinary_region: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            sealed_region: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            borrow: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            process_codec: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            list_element: false,
            ..capabilities
        },
        MemoryWitnessCapabilities {
            equality: false,
            ..capabilities
        },
    ] {
        let mut item = facts();
        item.capabilities = changed;
        candidates.push(item);
    }
    let mut item = facts();
    item.domain = MemoryWitnessDomain::SealedRegion;
    candidates.push(item);
    let mut item = facts();
    item.root = MemoryWitnessRoot::None;
    candidates.push(item);
    let mut item = facts();
    item.copy = MemoryWitnessCopy::SealedShare;
    candidates.push(item);
    let mut item = facts();
    item.drop = MemoryWitnessDrop::Structural;
    candidates.push(item);
    let mut item = facts();
    item.equality = MemoryWitnessEquality::List;
    candidates.push(item);
    let mut item = facts();
    item.codec = MemoryWitnessCodec::Ineligible;
    candidates.push(item);
    let mut item = facts();
    item.list_element = MemoryWitnessListElement::Copy;
    candidates.push(item);
    let mut item = facts();
    item.size = MemoryWitnessSize::Fixed(16);
    candidates.push(item);
    let mut item = facts();
    item.alignment = 16;
    candidates.push(item);
    let mut item = facts();
    item.contains_borrow = true;
    candidates.push(item);
    let mut item = facts();
    item.contains_dynamic_owner = false;
    candidates.push(item);
    let mut item = facts();
    item.portability = MemoryWitnessPortability::Portable;
    candidates.push(item);
    let mut item = facts();
    item.contention = MemoryWitnessContention::ImmutableShared;
    candidates.push(item);
    let mut item = facts();
    item.operations.push(MemoryWitnessOperation::Share);
    candidates.push(item);

    for candidate in candidates {
        assert_ne!(
            baseline,
            canonical_executable_memory_witness(&candidate, &[[3; 32]])
        );
    }
}

#[test]
fn capability_and_selected_routes_are_independently_compatible() {
    let mut ordinary = facts();
    ordinary.operations = required_memory_witness_operations(&ordinary);
    assert!(memory_witness_routes_are_compatible(&ordinary));

    let mut move_only = ordinary.clone();
    move_only.copy = MemoryWitnessCopy::Move;
    move_only.operations = required_memory_witness_operations(&move_only);
    assert!(!move_only
        .operations
        .contains(&MemoryWitnessOperation::Clone));
    assert!(memory_witness_routes_are_compatible(&move_only));

    let mut mode_crossing = ordinary.clone();
    mode_crossing.mode = MemoryWitnessMode::Copy;
    mode_crossing.operations = required_memory_witness_operations(&mode_crossing);
    assert!(memory_witness_routes_are_compatible(&mode_crossing));
    mode_crossing.contains_dynamic_owner = false;
    mode_crossing.operations = required_memory_witness_operations(&mode_crossing);
    assert!(!memory_witness_routes_are_compatible(&mode_crossing));

    let mut sealed = ordinary.clone();
    sealed.domain = MemoryWitnessDomain::SealedRegion;
    sealed.copy = MemoryWitnessCopy::SealedShare;
    sealed.contention = MemoryWitnessContention::ImmutableShared;
    sealed.operations = required_memory_witness_operations(&sealed);
    assert!(memory_witness_routes_are_compatible(&sealed));

    sealed.capabilities.sealed_region = false;
    sealed.operations = required_memory_witness_operations(&sealed);
    assert!(!memory_witness_routes_are_compatible(&sealed));
}
