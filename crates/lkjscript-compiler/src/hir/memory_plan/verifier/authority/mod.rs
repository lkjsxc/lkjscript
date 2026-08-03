use super::*;

mod call_signatures;
mod calls;
mod destinations;
mod entries;
mod escapes;
mod loans;
mod modes;
mod obligations;
mod witnesses;

use call_signatures::*;
use calls::*;
use destinations::*;
use entries::*;
use escapes::*;
use loans::*;
use modes::*;
use obligations::*;
use witnesses::*;

pub(super) fn verify_authority(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
) -> Result<u64> {
    let mut types = VerifiedTypes::new(program, plan)?;
    verify_authority_signatures(program, plan, &mut types)?;
    for entry in &plan.entries {
        let ty = authority_entry_type(program, facts, entry)?;
        let expected = types.intern(ty)?;
        if entry.type_fact != expected {
            return Err(Error::msg(
                "HIR memory entry has wrong authoritative type identity",
            ));
        }
        verify_entry_authority(entry, ty, types.expected(expected)?, facts)?;
    }
    verify_no_partial_or_affine_copy(facts, &mut types)?;
    verify_destinations(program, plan, facts, &types)?;
    verify_authority_calls(program, plan, facts, &mut types)?;
    types.verify_totals()?;
    verify_loans(plan, facts)?;
    verify_obligations(program, plan)?;
    let steps = plan
        .work
        .type_nodes
        .checked_add(plan.work.type_edges)
        .and_then(|value| value.checked_add(plan.work.scc_work))
        .and_then(|value| value.checked_add(plan.work.aggregate_fields))
        .and_then(|value| value.checked_add(plan.work.aggregate_variants))
        .and_then(|value| value.checked_add(plan.work.destinations))
        .and_then(|value| value.checked_add(plan.work.borrow_scopes))
        .and_then(|value| value.checked_add(plan.work.drop_paths))
        .ok_or_else(|| Error::msg("memory authority verifier work overflow"))?;
    Ok(steps)
}

fn verify_authority_signatures(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    types: &mut VerifiedTypes<'_>,
) -> Result<()> {
    for (index, function) in program.functions.iter().enumerate() {
        let actual = &plan
            .function(MemoryFunctionId::new(index_u32(index)?))
            .ok_or_else(|| Error::msg("memory authority lost function"))?
            .signature;
        let binding = program
            .binding(function.binding)
            .ok_or_else(|| Error::msg("memory authority lost callable"))?;
        let witness_parameters = verified_witness_parameters(&binding.ty, Some(&function.body))?;
        let mut parameters = Vec::new();
        for parameter in &function.params {
            let ty = &program
                .binding(*parameter)
                .ok_or_else(|| Error::msg("memory authority lost parameter"))?
                .ty;
            let dispose_parameter = match ty {
                Type::Param(name) => witness_parameters.iter().any(|requirement| {
                    requirement.parameter == *name
                        && requirement
                            .operations
                            .contains(&MemoryWitnessOperation::Dispose)
                }),
                _ => false,
            };
            parameters.push(if dispose_parameter {
                MemoryParameterMode::Consume
            } else {
                verified_parameter_mode(types, ty, resource_consumed(&function.body, *parameter))?
            });
        }
        let result = verified_result_mode(types, callable_result(&binding.ty)?)?;
        if u64::try_from(actual.witness_parameters.len()).unwrap_or(u64::MAX)
            > MAX_MEMORY_WITNESS_PARAMETERS
            || actual.witness_parameters != witness_parameters
            || actual.parameters != parameters
            || actual.result != result
        {
            return Err(Error::msg(
                "independent verifier rejected function memory signature",
            ));
        }
    }
    let main_id = MemoryFunctionId::new(index_u32(program.functions.len())?);
    let actual = &plan
        .function(main_id)
        .ok_or_else(|| Error::msg("memory authority lost main"))?
        .signature;
    let parameters = program
        .main
        .param_types
        .iter()
        .map(|ty| verified_parameter_mode(types, ty, false))
        .collect::<Result<Vec<_>>>()?;
    let result = verified_result_mode(types, &program.main.return_type)?;
    if !actual.witness_parameters.is_empty()
        || actual.parameters != parameters
        || actual.result != result
    {
        return Err(Error::msg(
            "independent verifier rejected main memory signature",
        ));
    }
    Ok(())
}

pub(super) fn verified_parameter_mode(
    types: &mut VerifiedTypes<'_>,
    ty: &Type,
    consumed: bool,
) -> Result<MemoryParameterMode> {
    if matches!(ty, Type::ByteSlice) {
        return Ok(MemoryParameterMode::BorrowShared);
    }
    if matches!(ty, Type::ByteSliceMut) {
        return Ok(MemoryParameterMode::BorrowExclusive);
    }
    if matches!(ty, Type::Resource(_)) {
        return Ok(if consumed {
            MemoryParameterMode::Consume
        } else {
            MemoryParameterMode::BorrowExclusive
        });
    }
    let id = types.intern(ty)?;
    let fact = types.expected(id)?;
    Ok(
        if fact.derived.closure.class != MemoryClosureClass::Deterministic
            || matches!(ty, Type::List(_))
        {
            MemoryParameterMode::Copy
        } else {
            match fact.derived.mode {
                MemoryAggregateMode::Copy => MemoryParameterMode::Copy,
                MemoryAggregateMode::ImmutableValue => MemoryParameterMode::BorrowShared,
                MemoryAggregateMode::Affine => MemoryParameterMode::Consume,
            }
        },
    )
}

pub(super) fn verified_result_mode(
    types: &mut VerifiedTypes<'_>,
    ty: &Type,
) -> Result<MemoryResultMode> {
    let id = types.intern(ty)?;
    let fact = types.expected(id)?;
    if fact.derived.contains_borrow {
        return Err(Error::msg(format!(
            "LKJ-MEM-BORROWED-RESULT type={:?}",
            verified_memory_type(ty)
        )));
    }
    if matches!(ty, Type::Resource(_)) {
        return Ok(MemoryResultMode::External);
    }
    Ok(
        if fact.derived.closure.class != MemoryClosureClass::Deterministic
            || fact.derived.mode == MemoryAggregateMode::Copy
            || matches!(ty, Type::List(_))
        {
            MemoryResultMode::Trivial
        } else {
            MemoryResultMode::Owned
        },
    )
}
