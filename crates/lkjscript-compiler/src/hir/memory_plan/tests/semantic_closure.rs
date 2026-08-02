#![allow(clippy::expect_used, clippy::panic)]

use super::super::*;
use super::fixtures::*;
use crate::hir;

fn witness<'a>(plan: &'a HirMemoryPlan, ty: &MemoryType) -> &'a MemoryWitness {
    let fact = fact(plan, ty).expect("type fact");
    plan.witnesses
        .iter()
        .find(|item| item.id == fact.witness)
        .expect("witness")
}

#[test]
fn unrelated_declaration_does_not_change_reachable_product_contract_or_witness() {
    let root = product(1, "root", &[("value", hir::Type::I64)]);
    let body = fake(hir::Type::Product(root.name.clone()));
    let baseline = derive(&program(
        hir::Type::Product(root.name.clone()),
        body.clone(),
        vec![root.clone()],
        Vec::new(),
    ))
    .expect("baseline plan");
    let unrelated = product(9, "unrelated", &[("other", hir::Type::Bool)]);
    let extended = derive(&program(
        hir::Type::Product(root.name.clone()),
        body,
        vec![root, unrelated],
        Vec::new(),
    ))
    .expect("extended plan");
    let left = witness(&baseline, &MemoryType::Product("root".into()));
    let right = witness(&extended, &MemoryType::Product("root".into()));
    assert_eq!(left.id, right.id);
    assert_eq!(left.facts.semantic_contract, right.facts.semantic_contract);
    assert_eq!(left.facts.semantic.declarations.len(), 1);
}

#[test]
fn product_field_roles_are_complete_ordered_and_identity_bearing() {
    let root = product(
        1,
        "pair",
        &[("left", hir::Type::I64), ("right", hir::Type::Bool)],
    );
    let body = fake(hir::Type::Product(root.name.clone()));
    let plan = derive(&program(
        hir::Type::Product(root.name.clone()),
        body,
        vec![root.clone()],
        Vec::new(),
    ))
    .expect("pair plan");
    let record = witness(&plan, &MemoryType::Product("pair".into()));
    assert_eq!(record.facts.dependencies.len(), 2);
    for (index, dependency) in record.facts.dependencies.iter().enumerate() {
        let lkjscript_contracts::ExecutableMemoryWitnessRole::ProductField {
            product,
            field,
            source_order,
        } = dependency.role
        else {
            panic!("product role")
        };
        assert_eq!(product, root.identity);
        assert_eq!(field, root.fields[index].identity);
        assert_eq!(usize::from(source_order), index);
        assert!(matches!(
            dependency.target,
            lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalWitness(_)
        ));
    }
}

#[test]
fn recursive_product_self_edge_uses_local_semantic_target() {
    let root = product(1, "tree", &[("next", hir::Type::Product("tree".into()))]);
    let body = fake(hir::Type::Product(root.name.clone()));
    let plan = derive(&program(
        hir::Type::Product(root.name.clone()),
        body,
        vec![root.clone()],
        Vec::new(),
    ))
    .expect("recursive tree plan");
    let record = witness(&plan, &MemoryType::Product("tree".into()));
    assert_eq!(record.facts.dependencies.len(), 1);
    assert!(matches!(record.facts.dependencies[0].target,
        lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalSemantic(id) if id == root.identity));
    assert_eq!(record.recompute_id().expect("recompute"), record.id);
}
