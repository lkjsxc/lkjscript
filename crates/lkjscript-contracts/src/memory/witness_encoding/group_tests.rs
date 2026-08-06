fn singleton_group() -> ExecutableMemoryWitnessGroup {
    let facts = facts();
    let mut member = ExecutableMemoryWitnessGroupMember {
        id: [0; 32], ordinal: 0, semantic_identity: facts.semantic_type,
        facts, dependencies: Vec::new(),
    };
    let id = executable_memory_witness_group_id(false, std::slice::from_ref(&member));
    member.id = executable_memory_witness_member_id(id, 0, member.semantic_identity);
    ExecutableMemoryWitnessGroup { id, recursive: false, members: vec![member] }
}

#[allow(clippy::expect_used)]
fn recursive_pair(local: bool) -> Vec<ExecutableMemoryWitnessGroup> {
    let declarations = vec![
        SemanticDeclaration::Product(SemanticProductDeclaration {
            identity: [1; 32], fields: vec![SemanticProductField {
                identity: [11; 32], source_order: 0, ty: SemanticType::Product([2; 32]),
            }],
        }),
        SemanticDeclaration::Product(SemanticProductDeclaration {
            identity: [2; 32], fields: vec![SemanticProductField {
                identity: [12; 32], source_order: 0, ty: SemanticType::Product([1; 32]),
            }],
        }),
    ];
    let mut members = Vec::new();
    for root in [[1; 32], [2; 32]] {
        let semantic = SemanticDescriptor {
            root: SemanticType::Product(root), declarations: declarations.clone(),
        };
        let mut member_facts = facts();
        member_facts.semantic_type = semantic_type_closure_hash(&semantic).expect("type identity");
        member_facts.semantic_contract = semantic_contract_hash(&semantic).expect("contract");
        member_facts.semantic = semantic;
        members.push(ExecutableMemoryWitnessGroupMember {
            id: [0; 32], ordinal: 0, semantic_identity: member_facts.semantic_type,
            facts: member_facts, dependencies: Vec::new(),
        });
    }
    members.sort_by_key(|member| member.semantic_identity);
    for (ordinal, member) in members.iter_mut().enumerate() {
        member.ordinal = u16::try_from(ordinal).expect("ordinal");
    }
    if local {
        for index in 0..members.len() {
            let requirement = semantic_dependency_requirements(&members[index].facts.semantic)
                .expect("pair dependency").remove(0);
            let target = members.iter().position(|member|
                member.facts.semantic.root == requirement.1).expect("pair target");
            members[index].dependencies.push(ExecutableMemoryWitnessDependency {
                role: requirement.0,
                target: ExecutableMemoryWitnessTarget::LocalMember(
                    u16::try_from(target).expect("target ordinal")),
            });
        }
        let id = executable_memory_witness_group_id(true, &members);
        for member in &mut members {
            member.id = executable_memory_witness_member_id(
                id, member.ordinal, member.semantic_identity);
        }
        return vec![ExecutableMemoryWitnessGroup { id, recursive: true, members }];
    }
    let group_ids = [[3; 32], [4; 32]];
    let member_ids = [[5; 32], [6; 32]];
    for index in 0..2 {
        let requirement = semantic_dependency_requirements(&members[index].facts.semantic)
            .expect("pair dependency").remove(0);
        let target = members.iter().position(|member|
            member.facts.semantic.root == requirement.1).expect("external pair target");
        members[index].dependencies.push(ExecutableMemoryWitnessDependency {
            role: requirement.0,
            target: ExecutableMemoryWitnessTarget::ExternalMember {
                group: group_ids[target], member: member_ids[target],
            },
        });
    }
    (0..2).map(|index| ExecutableMemoryWitnessGroup {
        id: group_ids[index], recursive: false,
        members: vec![ExecutableMemoryWitnessGroupMember {
            id: member_ids[index], ordinal: 0,
            semantic_identity: members[index].semantic_identity,
            facts: members[index].facts.clone(),
            dependencies: members[index].dependencies.clone(),
        }],
    }).collect()
}

#[test]
#[allow(clippy::expect_used)]
fn singleton_and_recursive_group_identities_validate_atomically() {
    validate_executable_memory_witness_groups(&[singleton_group()]).expect("singleton group");
    validate_executable_memory_witness_groups(&recursive_pair(true)).expect("recursive pair");
}

#[test]
#[allow(clippy::expect_used)]
fn witness_groups_cross_the_former_total_count_boundary() {
    const GROUPS: u64 = 16_385;
    let groups: Vec<_> = (0..GROUPS)
        .map(|index| {
            let mut identity = [0_u8; 32];
            identity[..8].copy_from_slice(&(index + 1).to_be_bytes());
            let semantic = SemanticDescriptor {
                root: SemanticType::Product(identity),
                declarations: vec![SemanticDeclaration::Product(SemanticProductDeclaration {
                    identity,
                    fields: Vec::new(),
                })],
            };
            let mut member_facts = facts();
            member_facts.semantic_type =
                semantic_type_closure_hash(&semantic).expect("semantic type identity");
            member_facts.semantic_contract =
                semantic_contract_hash(&semantic).expect("semantic contract identity");
            member_facts.semantic = semantic;
            let mut member = ExecutableMemoryWitnessGroupMember {
                id: [0; 32],
                ordinal: 0,
                semantic_identity: member_facts.semantic_type,
                facts: member_facts,
                dependencies: Vec::new(),
            };
            let id = executable_memory_witness_group_id(false, std::slice::from_ref(&member));
            member.id = executable_memory_witness_member_id(id, 0, member.semantic_identity);
            ExecutableMemoryWitnessGroup {
                id,
                recursive: false,
                members: vec![member],
            }
        })
        .collect();
    validate_executable_memory_witness_groups(&groups)
        .expect("witness group totals are not semantic admission limits");
}

#[test]
fn malformed_ordinal_reorder_and_forged_id_reject() {
    let mut ordinal = recursive_pair(true);
    ordinal[0].members[0].dependencies[0].target = ExecutableMemoryWitnessTarget::LocalMember(9);
    assert!(validate_executable_memory_witness_groups(&ordinal).is_err());
    let mut reordered = recursive_pair(true);
    reordered[0].members.swap(0, 1);
    assert!(validate_executable_memory_witness_groups(&reordered).is_err());
    let mut forged = singleton_group();
    forged.id = [9; 32];
    assert!(validate_executable_memory_witness_groups(&[forged]).is_err());
    let mut forged = singleton_group();
    forged.members[0].id = [9; 32];
    assert!(validate_executable_memory_witness_groups(&[forged]).is_err());
}

#[test]
#[allow(clippy::expect_used)]
fn external_group_cycle_rejects_before_identity_acceptance() {
    let error = validate_executable_memory_witness_groups(&recursive_pair(false))
        .expect_err("external cycle must reject");
    assert!(error.to_string().contains("cyclic"));
}
