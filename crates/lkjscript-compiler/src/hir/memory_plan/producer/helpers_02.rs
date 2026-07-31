type DomainAxes = (
    MemoryAliasing, MemoryDomain, MemoryDestruction, MemoryIdentity,
    MemoryPortability, MemoryContention, Option<&'static str>,
);

fn memory_mode(
    ty: &Type,
    fact: &MemoryTypeFact,
    effects: u16,
    escape: MemoryEscape,
) -> (MemoryMode, Option<&'static str>, MemoryExecution, Option<MemoryExecutionCutover>) {
    let multiplicity = if matches!(ty, Type::ByteSlice | Type::ByteSliceMut) {
        MemoryMultiplicity::Borrowed
    } else { match fact.mode {
        MemoryAggregateMode::Copy => MemoryMultiplicity::Copy,
        MemoryAggregateMode::ImmutableValue => MemoryMultiplicity::ImmutableValue,
        MemoryAggregateMode::Affine => MemoryMultiplicity::Affine,
    }};
    let (aliasing, domain, destruction, identity, portability, contention, family) = match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => (
            MemoryAliasing::Unique, MemoryDomain::Inline, MemoryDestruction::Trivial,
            MemoryIdentity::Value, MemoryPortability::Portable, MemoryContention::None, None,
        ),
        Type::Str => structural_domain(MemoryPortability::WorkerLocal),
        Type::Path => structural_domain(MemoryPortability::LinuxHost),
        Type::Bytes | Type::ByteVector => (
            MemoryAliasing::Unique, MemoryDomain::UniqueStructural, MemoryDestruction::DropGlue,
            MemoryIdentity::Value, MemoryPortability::WorkerLocal,
            MemoryContention::SingleOwner, None,
        ),
        Type::ByteSlice => borrowed_domain(false),
        Type::ByteSliceMut => borrowed_domain(true),
        Type::Symbol => static_domain(),
        Type::Resource(_) => (
            MemoryAliasing::External, MemoryDomain::ExternalResource,
            MemoryDestruction::ExternalClose, MemoryIdentity::ExternalResource,
            MemoryPortability::ProcessLocal, MemoryContention::ProviderSerialized, None,
        ),
        Type::Product(_) => aggregate_domain(
            fact,
            "product",
            MemoryPortability::WorkerLocal,
        ),
        Type::Enum { .. } => aggregate_domain(
            fact,
            "enum",
            MemoryPortability::WorkerLocal,
        ),
        Type::List(_) => legacy_domain("pair", MemoryPortability::WorkerLocal),
        Type::Fn { .. } | Type::Forall { .. } => static_domain(),
        Type::Param(_) => (
            MemoryAliasing::StaticShared, MemoryDomain::CallerDestination,
            MemoryDestruction::Trivial, MemoryIdentity::Value, MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared, None,
        ),
    };
    let execution_cutover = if fact.closure.class == MemoryClosureClass::Deterministic {
        execution_cutover(ty)
    } else { None };
    let execution = if execution_cutover.is_some() { MemoryExecution::CutoverRequired }
        else { MemoryExecution::Current };
    (MemoryMode { multiplicity, aliasing, escape, domain, destruction, identity, portability,
        contention, allocation_failure: allocation_failure(effects) }, family, execution,
        execution_cutover)
}

fn aggregate_domain(
    fact: &MemoryTypeFact,
    family: &'static str,
    portability: MemoryPortability,
) -> DomainAxes {
    if fact.closure.class == MemoryClosureClass::Deterministic {
        structural_domain(portability)
    } else {
        legacy_domain(family, portability)
    }
}

fn structural_domain(portability: MemoryPortability) -> DomainAxes {
    (
        MemoryAliasing::Unique,
        MemoryDomain::UniqueStructural,
        MemoryDestruction::DropGlue,
        MemoryIdentity::Value,
        portability,
        MemoryContention::SingleOwner,
        None,
    )
}

fn legacy_domain(family: &'static str, portability: MemoryPortability) -> DomainAxes {
    (MemoryAliasing::LegacyTracedShared, MemoryDomain::RegisteredLegacyTraced,
        MemoryDestruction::LegacyTraced, MemoryIdentity::Value, portability,
        MemoryContention::ImmutableShared, Some(family))
}

fn borrowed_domain(exclusive: bool) -> DomainAxes {
    (if exclusive { MemoryAliasing::BorrowedExclusive } else { MemoryAliasing::BorrowedShared },
        MemoryDomain::BorrowedView, MemoryDestruction::EndBorrow, MemoryIdentity::Value,
        MemoryPortability::WorkerLocal, MemoryContention::SingleOwner, None)
}

fn static_domain() -> DomainAxes {
    (MemoryAliasing::StaticShared, MemoryDomain::Static, MemoryDestruction::Trivial,
        MemoryIdentity::Value, MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared, None)
}

fn allocation_failure(effects: u16) -> MemoryAllocationFailure {
    let allocates = effects & crate::hir::EffectSet::ALLOCATES.bits() != 0;
    let trap = effects & crate::hir::EffectSet::MAY_TRAP.bits() != 0;
    let outcome = effects & crate::hir::EffectSet::MAY_EXIT.bits() != 0 || allocates;
    match (trap, outcome) {
        (false, false) => MemoryAllocationFailure::Impossible,
        (true, false) => MemoryAllocationFailure::Trap,
        (false, true) => MemoryAllocationFailure::StructuredOutcome,
        (true, true) => MemoryAllocationFailure::TrapOrOutcome,
    }
}

pub(crate) const fn resource_glue(kind: ResourceKind) -> MemoryDropGlueId {
    MemoryDropGlueId::new(1 + kind as u32)
}

const fn bytes_glue() -> MemoryDropGlueId {
    MemoryDropGlueId::new(1 + ResourceKind::ALL.len() as u32)
}

fn execution_cutover(ty: &Type) -> Option<MemoryExecutionCutover> {
    match ty {
        Type::Str => Some(MemoryExecutionCutover::StructuralString),
        Type::Path => Some(MemoryExecutionCutover::StructuralPath),
        Type::Product(name) => Some(MemoryExecutionCutover::Product(name.clone())),
        Type::Enum { id, arguments, .. } => Some(MemoryExecutionCutover::Enum {
            id: id.bytes(), arguments: arguments.iter().map(memory_type).collect(),
        }),
        _ => None,
    }
}
