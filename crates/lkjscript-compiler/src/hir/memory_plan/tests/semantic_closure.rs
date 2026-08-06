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
        assert_eq!(usize::try_from(source_order).ok(), Some(index));
        assert!(matches!(
            dependency.target,
            lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalMember { .. }
        ));
    }
}

#[test]
fn recursive_product_self_edge_uses_local_member_ordinal() {
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
    assert!(matches!(
        record.facts.dependencies[0].target,
        lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(0)
    ));
    let group = plan
        .witness_groups
        .iter()
        .find(|group| group.id == record.group)
        .expect("recursive witness group");
    assert!(group.recursive);
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].witness, record.id);
    assert_eq!(record.recompute_id().expect("recompute"), record.id);
}

#[test]
fn mutually_recursive_product_enum_closes_one_atomic_group() {
    let enumeration = enum_definition(
        40,
        "expression",
        &[],
        vec![("statement", vec![hir::Type::Product("statement".into())])],
    );
    let enumeration_ty = enum_type(&enumeration, Vec::new());
    let statement = product(41, "statement", &[("expression", enumeration_ty)]);
    let root = hir::Type::Product(statement.name.clone());
    let plan = derive(&program(
        root.clone(),
        fake(root),
        vec![statement],
        vec![enumeration],
    ))
    .expect("mutual recursive plan");
    let statement = witness(&plan, &MemoryType::Product("statement".into()));
    let group = plan
        .witness_groups
        .iter()
        .find(|group| group.id == statement.group)
        .expect("mutual recursive group");
    assert!(group.recursive);
    assert_eq!(group.members.len(), 2);
    assert!(group
        .members
        .windows(2)
        .all(|pair| pair[0].semantic_identity < pair[1].semantic_identity));
    assert!(group.members.iter().all(|member| plan
        .witness(member.witness)
        .is_some_and(|witness| witness.group == group.id)));
}

#[test]
fn generic_recursive_tree_instantiation_is_one_self_recursive_group() {
    let open_tree = hir::Type::Enum {
        id: enum_id(50),
        name: "tree".into(),
        arguments: vec![hir::Type::Param("t".into())],
    };
    let tree = enum_definition(
        50,
        "tree",
        &["t"],
        vec![
            ("leaf", vec![hir::Type::Param("t".into())]),
            ("branch", vec![open_tree]),
        ],
    );
    let closed = enum_type(&tree, vec![hir::Type::I64]);
    let plan = derive(&program(
        closed.clone(),
        fake(closed),
        Vec::new(),
        vec![tree],
    ))
    .expect("generic recursive tree plan");
    let record = witness(
        &plan,
        &MemoryType::Enum {
            id: enum_id(50).bytes(),
            name: "tree".into(),
            arguments: vec![MemoryType::I64],
        },
    );
    let group = plan
        .witness_groups
        .iter()
        .find(|group| group.id == record.group)
        .expect("generic recursive tree group");
    assert!(group.recursive);
    assert_eq!(group.members.len(), 1);
    assert!(record.facts.dependencies.iter().any(|dependency| matches!(
        dependency.target,
        lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(0)
    )));
}
