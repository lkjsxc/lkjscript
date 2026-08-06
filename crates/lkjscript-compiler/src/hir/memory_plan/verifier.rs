use std::collections::{BTreeMap, BTreeSet, HashMap};

use lkjscript_core::{Error, ResourceKind, Result};

use crate::hir::{self, BindingId, Type};

use super::*;

mod authority;
mod check;
mod drop_class;
mod expression_kind;
mod modes;
mod placement;
mod records;
mod support;
mod types;
mod walk;
mod walk_support;

use authority::*;
use check::*;
use drop_class::*;
use expression_kind::*;
use modes::*;
use placement::*;
use records::*;
use support::*;
use types::*;
use walk::*;
use walk_support::*;

pub(super) fn verify(program: &hir::Program, plan: &HirMemoryPlan) -> Result<u64> {
    if plan.schema != HIR_MEMORY_PLAN_SCHEMA || compute_plan_id(plan)? != plan.id {
        return Err(Error::msg(
            "independent HIR memory-plan verifier rejected content identity",
        ));
    }
    let facts = collect(program)?;
    verify_dense(plan)?;
    verify_work(plan, &facts)?;
    verify_functions(program, plan)?;
    verify_expressions(plan, &facts)?;
    verify_uses_and_constants(plan, &facts)?;
    verify_entries(plan)?;
    let placement_steps = verify_value_placements(program, plan, &facts)?;
    verify_drop_glues(plan)?;
    verify_drop_classes(program, plan)?;
    let authority_steps = verify_authority(program, plan, &facts)?;
    let steps = facts
        .steps
        .checked_add(plan.work.entries)
        .and_then(|value| value.checked_add(plan.work.uses))
        .and_then(|value| value.checked_add(plan.work.obligations))
        .and_then(|value| value.checked_add(placement_steps))
        .and_then(|value| value.checked_add(authority_steps))
        .ok_or_else(|| Error::msg("independent HIR memory-plan verifier work overflow"))?;
    Ok(steps)
}

fn verify_functions(program: &hir::Program, plan: &HirMemoryPlan) -> Result<()> {
    if plan.functions.len() != program.functions.len().saturating_add(1) {
        return Err(Error::msg("HIR memory plan does not cover every function"));
    }
    for (index, function) in program.functions.iter().enumerate() {
        let id = MemoryFunctionId::new(index_u32(index)?);
        let actual = plan
            .function(id)
            .ok_or_else(|| Error::msg("HIR memory plan is missing a dense function identity"))?;
        let binding = program
            .binding(function.binding)
            .ok_or_else(|| Error::msg("HIR memory verifier found unknown function binding"))?;
        if actual.binding != Some(function.binding.raw())
            || actual.name != binding.name
            || actual.source != function.origin.raw()
            || actual.signature.function != id
            || actual.signature.parameters.len() != function.params.len()
        {
            return Err(Error::msg(
                "HIR memory function identity/signature mismatch",
            ));
        }
    }
    let main_id = MemoryFunctionId::new(index_u32(program.functions.len())?);
    let main = plan
        .function(main_id)
        .ok_or_else(|| Error::msg("HIR memory plan is missing main"))?;
    if main.name != "main"
        || main.binding.is_some()
        || main.source != program.main.origin.raw()
        || main.signature.parameters.len() != program.main.param_types.len()
    {
        return Err(Error::msg("HIR main memory signature mismatch"));
    }
    Ok(())
}

fn verify_drop_glues(plan: &HirMemoryPlan) -> Result<()> {
    if plan.drop_glues.len() < ResourceKind::ALL.len().saturating_add(2)
        || plan.drop_glues.first().map(|glue| glue.kind.clone())
            != Some(MemoryDropGlueKind::ByteVector)
    {
        return Err(Error::msg("HIR memory-plan drop-glue table is incomplete"));
    }
    for (index, kind) in ResourceKind::ALL.into_iter().enumerate() {
        let expected = MemoryDropGlueId::new(1 + u32::from(kind as u8));
        let glue = plan.drop_glues.get(index.saturating_add(1));
        if glue.map(|glue| (glue.id, glue.kind.clone()))
            != Some((expected, MemoryDropGlueKind::Resource(kind)))
        {
            return Err(Error::msg("HIR memory-plan resource drop glue mismatch"));
        }
    }
    let bytes_id = MemoryDropGlueId::new(1 + ResourceKind::ALL.len() as u32);
    let bytes = plan.drop_glues.get(bytes_id.index().unwrap_or(usize::MAX));
    if bytes.map(|glue| (glue.id, glue.kind.clone())) != Some((bytes_id, MemoryDropGlueKind::Bytes))
    {
        return Err(Error::msg("HIR memory-plan bytes drop glue mismatch"));
    }
    Ok(())
}

fn index_u32(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(|_| Error::msg("HIR memory verifier index exceeds u32"))
}
