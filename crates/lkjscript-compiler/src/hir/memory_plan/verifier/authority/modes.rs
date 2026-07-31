use super::*;

pub(super) fn verify_entry_authority(
    entry: &MemoryPlanEntry,
    ty: &Type,
    fact: &VerifiedExpectedType,
    facts: &Facts<'_>,
) -> Result<()> {
    let escape = expected_entry_escape(entry, facts)?;
    let multiplicity = if matches!(ty, Type::ByteSlice | Type::ByteSliceMut) {
        MemoryMultiplicity::Borrowed
    } else {
        match fact.derived.mode {
            MemoryAggregateMode::Copy => MemoryMultiplicity::Copy,
            MemoryAggregateMode::ImmutableValue => MemoryMultiplicity::ImmutableValue,
            MemoryAggregateMode::Affine => MemoryMultiplicity::Affine,
        }
    };
    let (aliasing, domain, destruction, identity, portability, contention) =
        verified_domain_axes(ty, fact)?;
    let mut mode = MemoryMode {
        multiplicity,
        aliasing,
        escape,
        domain,
        destruction,
        identity,
        portability,
        contention,
        allocation_failure: verified_allocation_failure(entry.effects),
    };
    let static_bytes = is_static_bytes(entry, facts);
    if static_bytes {
        mode.multiplicity = MemoryMultiplicity::Copy;
        mode.aliasing = MemoryAliasing::StaticShared;
        mode.domain = MemoryDomain::Static;
        mode.destruction = MemoryDestruction::Trivial;
        mode.contention = MemoryContention::ImmutableShared;
    }
    let execution_cutover = if fact.derived.closure.class == MemoryClosureClass::Deterministic {
        verified_execution_cutover(ty)
    } else {
        None
    };
    let execution = if execution_cutover.is_some() || domain == MemoryDomain::UnsupportedRuntime {
        MemoryExecution::CutoverRequired
    } else {
        MemoryExecution::Current
    };
    let owns_glue = fact.derived.mode != MemoryAggregateMode::Copy && !static_bytes;
    let root = if static_bytes {
        MemoryRootProjection::None
    } else {
        if fact.derived.closure.class == MemoryClosureClass::RegionClosed {
            MemoryRootProjection::None
        } else if fact.derived.contains_dynamic_owner
            || matches!(ty, Type::Str | Type::Path)
            || (matches!(ty, Type::Product(_) | Type::Enum { .. })
                && fact.derived.closure.class == MemoryClosureClass::Deterministic)
        {
            MemoryRootProjection::Structural
        } else {
            MemoryRootProjection::None
        }
    };
    let copy_share = if static_bytes {
        MemoryCopySharePlan::StaticIdentity
    } else {
        fact_copy_share(ty, fact)
    };
    if entry.mode != mode
        || entry.execution != execution
        || entry.execution_cutover != execution_cutover
        || entry.root_projection != root
        || (entry.borrow_scope.is_none() && entry.copy_share != copy_share)
        || entry.drop_glue != owns_glue.then_some(fact.glue).flatten()
        || entry.drop_path != owns_glue.then_some(fact.path).flatten()
    {
        return Err(Error::msg(
            "independent verifier rejected entry memory authority axes",
        ));
    }
    Ok(())
}

pub(super) fn verified_execution_cutover(ty: &Type) -> Option<MemoryExecutionCutover> {
    match ty {
        Type::Str => Some(MemoryExecutionCutover::StructuralString),
        Type::Path => Some(MemoryExecutionCutover::StructuralPath),
        Type::Product(name) => Some(MemoryExecutionCutover::Product(name.clone())),
        Type::Enum { id, arguments, .. } => Some(MemoryExecutionCutover::Enum {
            id: id.bytes(),
            arguments: arguments.iter().map(verified_memory_type).collect(),
        }),
        _ => None,
    }
}

fn fact_copy_share(ty: &Type, fact: &VerifiedExpectedType) -> MemoryCopySharePlan {
    verified_copy_share(ty, &fact.derived)
}

include!("modes/axes.rs");
