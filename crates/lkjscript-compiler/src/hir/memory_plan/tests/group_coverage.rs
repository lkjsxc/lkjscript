#![allow(clippy::expect_used)]

use super::super::*;
use super::fixtures::*;
use crate::hir;

#[test]
fn mutually_recursive_expression_statement_enums_share_one_group() {
    let expression_ty = hir::Type::Enum {
        id: enum_id(60),
        arguments: Vec::new(),
    };
    let statement_ty = hir::Type::Enum {
        id: enum_id(70),
        arguments: Vec::new(),
    };
    let expression = enum_definition(
        60,
        "expression",
        &[],
        vec![("statement", vec![statement_ty.clone()])],
    );
    let statement = enum_definition(
        70,
        "statement",
        &[],
        vec![("expression", vec![expression_ty.clone()])],
    );
    let plan = derive(&program(
        expression_ty.clone(),
        fake(expression_ty),
        Vec::new(),
        vec![expression, statement],
    ))
    .expect("mutual enum plan");
    let groups: Vec<_> = plan
        .witness_groups
        .iter()
        .filter(|group| group.recursive && group.members.len() == 2)
        .collect();
    assert_eq!(groups.len(), 1);
    assert!(groups[0].members.iter().all(|member| plan
        .witness(member.witness)
        .is_some_and(|witness| witness.group == Some(groups[0].id))));
}

#[test]
fn parent_group_names_exact_external_child_group_and_member() {
    let child = product(0, "child", &[("value", hir::Type::I64)]);
    let parent = product(1, "parent", &[("child", hir::Type::Product(child.id))]);
    let root = hir::Type::Product(parent.id);
    let plan = derive(&program(
        root.clone(),
        fake(root),
        vec![child, parent],
        Vec::new(),
    ))
    .expect("external child group plan");
    let parent = super::witnesses::witness(&plan, &MemoryType::Product(hir::ProductId::new(1)))
        .expect("parent witness");
    let child = super::witnesses::witness(&plan, &MemoryType::Product(hir::ProductId::new(0)))
        .expect("child witness");
    assert!(matches!(parent.facts.dependencies[0].target,
        lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalMember { group, member }
            if group == child.group.expect("child group").as_bytes()
                && member == child.id.as_bytes()));
}
