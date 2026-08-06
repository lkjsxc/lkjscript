use super::*;

fn facts() -> ExecutableMemoryWitnessFacts {
    let semantic = SemanticDescriptor {
        root: SemanticType::Primitive(SemanticPrimitiveKind::Unit),
        declarations: Vec::new(),
    };
    let semantic_contract = semantic_contract_hash(&semantic).unwrap_or([0; 32]);
    let semantic_type = semantic_type_closure_hash(&semantic).unwrap_or([0; 32]);
    ExecutableMemoryWitnessFacts {
        semantic_type,
        semantic_contract,
        semantic,
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

fn dependency(byte: u8, source_order: u64) -> ExecutableMemoryWitnessDependency {
    ExecutableMemoryWitnessDependency {
        role: ExecutableMemoryWitnessRole::ProductField {
            product: [9; 32],
            field: [byte; 32],
            source_order,
        },
        target: ExecutableMemoryWitnessTarget::ExternalMember {
            group: [byte.saturating_add(1); 32],
            member: [byte; 32],
        },
    }
}

fn group_descriptor(
    facts: ExecutableMemoryWitnessFacts,
    dependencies: Vec<ExecutableMemoryWitnessDependency>,
) -> Vec<u8> {
    let semantic_identity = facts.semantic_type;
    canonical_executable_memory_witness_group_descriptor(
        false,
        &[ExecutableMemoryWitnessGroupMember {
            id: [0; 32],
            ordinal: 0,
            semantic_identity,
            facts,
            dependencies,
        }],
    )
}

#[test]
fn executable_witness_group_encoding_is_deterministic_and_dependency_ordered() {
    let dependencies = vec![dependency(3, 0), dependency(4, 1)];
    let first = group_descriptor(facts(), dependencies.clone());
    let second = group_descriptor(facts(), dependencies);
    assert_eq!(first, second);
    assert_ne!(
        first,
        group_descriptor(facts(), vec![dependency(4, 1), dependency(3, 0)])
    );
    assert_ne!(first, group_descriptor(facts(), vec![dependency(3, 0)]));
}

#[test]
fn every_executable_fact_changes_the_canonical_encoding() {
    let baseline = group_descriptor(facts(), vec![dependency(3, 0)]);
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
            group_descriptor(candidate, vec![dependency(3, 0)])
        );
    }
}

include!("witness_encoding/group_tests.rs");
include!("witness_encoding/route_tests.rs");
