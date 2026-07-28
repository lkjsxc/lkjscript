#![allow(clippy::panic)]

use lkjscript_core::Limits;

#[test]
fn executable_retains_dense_independently_verified_hir_memory_plan() {
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let first = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default());
    let second = crate::compile_source(source, "memory-plan.lkjscript", &Limits::default());
    let (Ok(first), Ok(second)) = (first, second) else {
        panic!("memory-plan fixture must compile");
    };
    let plan = first.memory_plan();
    assert_eq!(plan.schema, super::HIR_MEMORY_PLAN_SCHEMA);
    assert_eq!(plan.id, second.memory_plan().id);
    assert_eq!(plan.functions.len(), 1);
    assert_eq!(plan.work.expressions, 1);
    assert!(plan.work.verifier_steps > 0);
    assert!(plan
        .entries
        .iter()
        .enumerate()
        .all(|(index, entry)| entry.id.raw() == u32::try_from(index).unwrap_or(u32::MAX)));
}
