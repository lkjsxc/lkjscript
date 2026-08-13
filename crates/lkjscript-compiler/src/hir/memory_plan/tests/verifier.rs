use super::super::*;
use super::call_fixture::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::{Error, Result};

fn destination_fixture() -> Result<(hir::Program, HirMemoryPlan)> {
    let record = product(0, "verified-record", &[("name", hir::Type::Str)]);
    let ty = hir::Type::Product(record.id);
    let program = program(
        ty,
        product_value(&record, vec![text("name")]),
        vec![record],
        Vec::new(),
    );
    let plan = producer::derive(&program)?;
    Ok((program, plan))
}

fn rejected(program: &hir::Program, mut plan: HirMemoryPlan) -> Result<bool> {
    Ok(verify_forged(program, &mut plan).is_err())
}

#[test]
fn corrupted_mode_closure_destination_and_glue_are_independently_rejected() -> Result<()> {
    let (program, plan) = destination_fixture()?;
    if plan.entries.is_empty() {
        return Err(Error::msg("destination fixture failed"));
    }

    let mut mode = plan.clone();
    mode.entries[0].mode.multiplicity = MemoryMultiplicity::Affine;
    assert!(rejected(&program, mode)?);

    let mut closure = plan.clone();
    closure
        .type_facts
        .last_mut()
        .ok_or_else(|| Error::msg("type fact missing"))?
        .closure
        .class = MemoryClosureClass::Unresolved;
    assert!(rejected(&program, closure)?);

    let mut destination = plan.clone();
    destination
        .destinations
        .first_mut()
        .ok_or_else(|| Error::msg("destination missing"))?
        .kind = MemoryDestinationKind::Stack;
    assert!(rejected(&program, destination)?);

    let mut group = plan.clone();
    group.witness_groups[0].members[0].ordinal = 1;
    assert!(rejected(&program, group)?);

    let mut glue = plan.clone();
    let structural = glue
        .drop_glues
        .iter_mut()
        .find(|item| item.drop_path.is_some())
        .ok_or_else(|| Error::msg("structural glue missing"))?;
    structural.kind = MemoryDropGlueKind::Path;
    assert!(rejected(&program, glue)?);
    Ok(())
}

#[test]
fn corrupted_direct_borrow_scope_is_independently_rejected() -> Result<()> {
    let program = direct_call_program(hir::Type::Str, text("borrow"), Vec::new());
    let plan = producer::derive(&program)?;
    let mut scope = plan.clone();
    scope
        .borrow_scopes
        .first_mut()
        .ok_or_else(|| Error::msg("borrow scope missing"))?
        .semantic_uses = 2;
    assert!(rejected(&program, scope)?);

    let mut call = plan.clone();
    call.calls
        .iter_mut()
        .find(|item| matches!(item.target, MemoryCallTarget::Direct(_)))
        .ok_or_else(|| Error::msg("direct call missing"))?
        .borrow_scopes[0] = None;
    assert!(rejected(&program, call)?);
    Ok(())
}

#[test]
fn every_aggregate_authority_fact_changes_plan_identity() -> Result<()> {
    let program = direct_call_program(hir::Type::Str, text("identity"), Vec::new());
    let plan = producer::derive(&program)?;
    let original = plan.id;
    let mut changed = Vec::new();

    let mut entry = plan.clone();
    entry.entries[0].root_projection = MemoryRootProjection::None;
    changed.push(entry);
    let mut cutover = plan.clone();
    cutover.entries[0].execution_cutover = None;
    changed.push(cutover);
    let mut fact = plan.clone();
    fact.type_facts
        .iter_mut()
        .find(|item| item.ty == MemoryType::String)
        .ok_or_else(|| Error::msg("string fact missing"))?
        .copy_share = MemoryCopySharePlan::Move;
    changed.push(fact);
    let mut borrow = plan.clone();
    borrow
        .borrow_scopes
        .first_mut()
        .ok_or_else(|| Error::msg("scope missing"))?
        .end_after = MemoryExpressionId::new(0);
    changed.push(borrow);
    let mut path = plan.clone();
    path.drop_paths
        .first_mut()
        .ok_or_else(|| Error::msg("drop path missing"))?
        .branches
        .push(MemoryDropBranch {
            active_variant: None,
            actions: Vec::new(),
        });
    changed.push(path);
    let mut work = plan.clone();
    work.work.type_edges = work.work.type_edges.saturating_add(1);
    changed.push(work);

    for candidate in &changed {
        assert_ne!(compute_plan_id(candidate)?, original);
    }
    Ok(())
}
