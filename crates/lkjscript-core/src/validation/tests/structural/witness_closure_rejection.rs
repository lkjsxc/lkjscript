#[test]
fn bytecode_rejects_forged_local_semantic_target() {
    use lkjscript_contracts::{
        ExecutableMemoryWitnessDependency as Dependency, ExecutableMemoryWitnessRole as Role,
        ExecutableMemoryWitnessTarget as Target, SemanticPrimitiveKind as Primitive,
        SemanticType,
    };
    let mut chunk = witness_chunk();
    let mut facts = scalar_facts();
    facts.semantic.root = SemanticType::List(Box::new(SemanticType::Primitive(Primitive::I64)));
    facts.semantic_contract = lkjscript_contracts::semantic_contract_hash(&facts.semantic)
        .expect("list semantic contract");
    chunk.memory_witnesses[0] = scalar_witness(
        facts,
        vec![Dependency {
            role: Role::ListElement,
            target: Target::LocalMember(9),
        }],
    );
    install_group(&mut chunk);
    assert!(error(chunk).contains("recursive group classification is invalid"));
}

#[test]
fn bytecode_rejects_duplicate_semantic_type_owner_representation() {
    let mut chunk = witness_chunk();
    let first = chunk.memory_witnesses[0].clone();
    let mut second_facts = scalar_facts();
    second_facts.portability = lkjscript_contracts::MemoryWitnessPortability::WorkerLocal;
    let second = scalar_witness(second_facts, Vec::new());
    chunk.memory_witnesses = vec![first, second];
    chunk.memory_witnesses.sort_by_key(|item| item.id);
    install_group(&mut chunk);
    assert!(error(chunk).contains("duplicate bytecode semantic type and owner representation"));
}

#[test]
fn bytecode_rejects_swapped_complete_product_roles() {
    use lkjscript_contracts::{
        ExecutableMemoryWitnessDependency as Dependency, ExecutableMemoryWitnessTarget as Target,
        SemanticDeclaration, SemanticPrimitiveKind as Primitive, SemanticProductDeclaration,
        SemanticProductField, SemanticType,
    };
    let mut chunk = witness_chunk();
    let mut facts = scalar_facts();
    facts.semantic.root = SemanticType::Product([7; 32]);
    facts.semantic.declarations = vec![SemanticDeclaration::Product(SemanticProductDeclaration {
        identity: [7; 32],
        fields: vec![
            SemanticProductField { identity: [8; 32], source_order: 0, ty: SemanticType::Primitive(Primitive::I64) },
            SemanticProductField { identity: [9; 32], source_order: 1, ty: SemanticType::Primitive(Primitive::I64) },
        ],
    })];
    facts.semantic_contract = lkjscript_contracts::semantic_contract_hash(&facts.semantic)
        .expect("product semantic contract");
    let requirements = lkjscript_contracts::semantic_dependency_requirements(&facts.semantic)
        .expect("product roles");
    chunk.memory_witnesses[0] = scalar_witness(facts, vec![
        Dependency { role: requirements[1].0.clone(), target: Target::ExternalMember {
            group: [4; 32], member: [2; 32] } },
        Dependency { role: requirements[0].0.clone(), target: Target::ExternalMember {
            group: [5; 32], member: [3; 32] } },
    ]);
    install_group(&mut chunk);
    assert!(error(chunk).contains("dependency closure is invalid"));
}
