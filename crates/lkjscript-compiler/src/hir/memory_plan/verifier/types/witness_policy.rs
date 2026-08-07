fn verified_witness_requirement(ty: &Type) -> MemoryWitnessRequirement {
    if matches!(ty, Type::Param(_)) {
        MemoryWitnessRequirement::SpecializationRequired
    } else {
        MemoryWitnessRequirement::Concrete
    }
}

fn verified_witness_domain(ty: &Type, derived: &VerifiedDerived) -> MemoryDomain {
    match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => {
            MemoryDomain::Inline
        }
        Type::Symbol | Type::Fn { .. } | Type::Forall { .. } => MemoryDomain::Static,
        Type::Str | Type::Path | Type::Bytes | Type::ByteVector => MemoryDomain::UniqueStructural,
        Type::ByteSlice | Type::ByteSliceMut => MemoryDomain::BorrowedView,
        Type::Resource(_) => MemoryDomain::ExternalResource,
        Type::Param(_) => MemoryDomain::CallerDestination,
        Type::List(_) if derived.closure.class == MemoryClosureClass::RegionClosed => {
            MemoryDomain::OrdinaryRegion
        }
        Type::List(_) => MemoryDomain::UnsupportedRuntime,
        Type::Product(_) | Type::Enum { .. }
            if derived.closure.class == MemoryClosureClass::RegionClosed =>
        {
            MemoryDomain::OrdinaryRegion
        }
        Type::Enum { .. } if derived.closure.class == MemoryClosureClass::Deterministic => {
            MemoryDomain::UniqueStructural
        }
        Type::Product(_) if derived.closure.class == MemoryClosureClass::Deterministic => {
            MemoryDomain::UniqueStructural
        }
        Type::Product(_) | Type::Enum { .. } => MemoryDomain::UnsupportedRuntime,
    }
}

fn verified_witness_equality(ty: &Type) -> MemoryEqualitySupport {
    match ty {
        Type::Param(_) => MemoryEqualitySupport::CallerWitnessRequired,
        Type::List(inner) if verified_value_equality(inner) => MemoryEqualitySupport::EqualList,
        _ if verified_value_equality(ty) => MemoryEqualitySupport::EqualValue,
        _ => MemoryEqualitySupport::Unsupported,
    }
}

fn verified_value_equality(ty: &Type) -> bool {
    match ty {
        Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Path | Type::Symbol => {
            true
        }
        Type::Enum { id, arguments, .. }
            if matches!(
                id.bytes(),
                lkjscript_core::OPTION_ID | lkjscript_core::RESULT_ID
            ) =>
        {
            arguments.iter().all(verified_value_equality)
        }
        _ => false,
    }
}

fn verified_witness_semantic_snapshot(
    ty: &Type,
    derived: &VerifiedDerived,
) -> MemorySemanticSnapshotEligibility {
    if matches!(ty, Type::Param(_)) {
        return MemorySemanticSnapshotEligibility::CallerWitnessRequired;
    }
    if (matches!(ty, Type::Product(_) | Type::Enum { .. })
        && derived.closure.class == MemoryClosureClass::RegionClosed)
        || !matches!(
            derived.closure.class,
            MemoryClosureClass::Deterministic | MemoryClosureClass::RegionClosed
        )
        || derived.mode == MemoryAggregateMode::Affine
        || derived.contains_borrow
        || !verified_witness_snapshot_type(ty)
    {
        MemorySemanticSnapshotEligibility::Ineligible
    } else {
        MemorySemanticSnapshotEligibility::Eligible
    }
}

fn verified_witness_snapshot_type(ty: &Type) -> bool {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Never
            | Type::Capability(_)
            | Type::Resource(_)
            | Type::Fn { .. }
            | Type::Forall { .. } => return false,
            Type::Enum { arguments, .. } => pending.extend(arguments),
            Type::List(inner) => pending.push(inner),
            _ => {}
        }
    }
    true
}

pub(super) fn verified_witness_list_element(
    ty: &Type,
    derived: &VerifiedDerived,
) -> MemoryListElementEligibility {
    if matches!(ty, Type::Param(_)) {
        return MemoryListElementEligibility::CallerWitnessRequired;
    }
    if matches!(ty, Type::List(_))
        && derived.mode == MemoryAggregateMode::ImmutableValue
        && derived.closure.class == MemoryClosureClass::RegionClosed
        && !derived.contains_borrow
        && !derived.contains_dynamic_owner
    {
        return MemoryListElementEligibility::Copy;
    }
    if derived.closure.class != MemoryClosureClass::Deterministic {
        return MemoryListElementEligibility::UnsupportedUnresolved;
    }
    if derived.contains_borrow {
        return MemoryListElementEligibility::UnsupportedBorrow;
    }
    match derived.mode {
        MemoryAggregateMode::Copy => MemoryListElementEligibility::Copy,
        MemoryAggregateMode::ImmutableValue => MemoryListElementEligibility::ImmutableValue,
        MemoryAggregateMode::Affine => MemoryListElementEligibility::UnsupportedAffine,
    }
}

fn verified_witness_dynamic_size(ty: &Type) -> MemoryDynamicSize {
    match ty {
        Type::Param(_) => MemoryDynamicSize::CallerWitnessRequired,
        Type::Str
        | Type::Bytes
        | Type::Path
        | Type::ByteVector
        | Type::List(_)
        | Type::Product(_)
        | Type::Enum { .. } => MemoryDynamicSize::Dynamic,
        _ => MemoryDynamicSize::Fixed,
    }
}

fn verified_witness_portability(ty: &Type) -> MemoryPortability {
    match ty {
        Type::Path => MemoryPortability::LinuxHost,
        Type::Resource(_) => MemoryPortability::ProcessLocal,
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => {
            MemoryPortability::Portable
        }
        _ => MemoryPortability::WorkerLocal,
    }
}

fn verified_witness_contention(ty: &Type, derived: &VerifiedDerived) -> MemoryContention {
    match ty {
        Type::Resource(_) => MemoryContention::ProviderSerialized,
        Type::Symbol | Type::Fn { .. } | Type::Forall { .. } | Type::Param(_) => {
            MemoryContention::ImmutableShared
        }
        Type::List(_) if derived.closure.class == MemoryClosureClass::RegionClosed => {
            MemoryContention::ImmutableShared
        }
        Type::List(_) => MemoryContention::UnresolvedShared,
        _ if derived.contains_dynamic_owner
            || matches!(
                derived.mode,
                MemoryAggregateMode::Affine | MemoryAggregateMode::ImmutableValue
            ) =>
        {
            MemoryContention::SingleOwner
        }
        _ => MemoryContention::None,
    }
}
