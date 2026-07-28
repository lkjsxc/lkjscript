use std::collections::{BTreeMap, BTreeSet};

use lkjscript_core::{Error, ResourceKind, Result};

use crate::hir::{self, BindingId, Type};

use super::{
    compute_plan_id, HirMemoryPlan, MemoryDropGlueId, MemoryDropGlueKind, MemoryEscape,
    MemoryFunctionId, MemoryParameterMode, MemoryStorage, MemorySubject, MemoryType,
    HIR_MEMORY_PLAN_SCHEMA, MAX_MEMORY_PLAN_VERIFIER_STEPS,
};

mod check;
mod support;
mod walk;
mod walk_support;

use check::*;
use support::*;
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
    verify_entries(plan)?;
    verify_legacy_registration(plan)?;
    verify_drop_glues(plan)?;
    verify_calls(program, plan, &facts)?;
    let steps = facts
        .steps
        .checked_add(plan.work.entries)
        .and_then(|value| value.checked_add(plan.work.uses))
        .and_then(|value| value.checked_add(plan.work.obligations))
        .ok_or_else(|| Error::msg("independent HIR memory-plan verifier work overflow"))?;
    if steps > MAX_MEMORY_PLAN_VERIFIER_STEPS {
        return Err(Error::msg(format!(
            "independent HIR memory-plan verifier work exceeds {MAX_MEMORY_PLAN_VERIFIER_STEPS}"
        )));
    }
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
        for (position, parameter) in function.params.iter().enumerate() {
            let ty = &program
                .binding(*parameter)
                .ok_or_else(|| Error::msg("HIR memory verifier found unknown parameter"))?
                .ty;
            let expected = parameter_mode(ty, resource_consumed(&function.body, *parameter));
            if actual.signature.parameters.get(position) != Some(&expected) {
                return Err(Error::msg("HIR memory parameter mode mismatch"));
            }
        }
        let result = callable_result(&binding.ty)?;
        if actual.signature.result != result_mode(result) {
            return Err(Error::msg("HIR memory result mode mismatch"));
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
        || main.signature.result != result_mode(&program.main.return_type)
    {
        return Err(Error::msg("HIR main memory signature mismatch"));
    }
    Ok(())
}

fn verify_legacy_registration(plan: &HirMemoryPlan) -> Result<()> {
    let registered: BTreeSet<&str> = lkjscript_contracts::LEGACY_TRACED_FAMILIES
        .iter()
        .map(|family| family.identity)
        .collect();
    for entry in &plan.entries {
        let expected = legacy_family(&entry.ty);
        if entry.legacy_family.as_deref() != expected {
            return Err(Error::msg("HIR memory plan has a wrong legacy family"));
        }
        if let Some(family) = expected {
            if !registered.contains(family) || entry.mode.storage != MemoryStorage::LegacyTraced {
                return Err(Error::msg(
                    "HIR memory plan selected unregistered legacy tracing",
                ));
            }
        } else if entry.mode.storage == MemoryStorage::LegacyTraced {
            return Err(Error::msg(
                "HIR memory plan selected legacy tracing without an exact family",
            ));
        }
    }
    Ok(())
}

fn verify_drop_glues(plan: &HirMemoryPlan) -> Result<()> {
    if plan.drop_glues.len() != ResourceKind::ALL.len().saturating_add(1)
        || plan.drop_glues.first().map(|glue| glue.kind)
            != Some(MemoryDropGlueKind::LegacyTracedByteVector)
    {
        return Err(Error::msg("HIR memory-plan drop-glue table is incomplete"));
    }
    for (index, kind) in ResourceKind::ALL.into_iter().enumerate() {
        let expected = MemoryDropGlueId::new(1 + u32::from(kind as u8));
        let glue = plan.drop_glues.get(index.saturating_add(1));
        if glue.map(|glue| (glue.id, glue.kind))
            != Some((expected, MemoryDropGlueKind::Resource(kind)))
        {
            return Err(Error::msg("HIR memory-plan resource drop glue mismatch"));
        }
    }
    Ok(())
}

fn index_u32(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(|_| Error::msg("HIR memory verifier index exceeds u32"))
}
