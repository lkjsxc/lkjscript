type DomainAxes = (
    MemoryAliasing, MemoryDomain, MemoryDestruction, MemoryIdentity,
    MemoryPortability, MemoryContention,
);

fn memory_mode(
    ty: &Type,
    fact: &MemoryTypeFact,
    effects: u16,
    escape: MemoryEscape,
) -> Result<(MemoryMode, MemoryExecution, Option<MemoryExecutionCutover>)> {
    let multiplicity = if matches!(ty, Type::ByteSlice | Type::ByteSliceMut) {
        MemoryMultiplicity::Borrowed
    } else { match fact.mode {
        MemoryAggregateMode::Copy => MemoryMultiplicity::Copy,
        MemoryAggregateMode::ImmutableValue => MemoryMultiplicity::ImmutableValue,
        MemoryAggregateMode::Affine => MemoryMultiplicity::Affine,
    }};
    let (aliasing, domain, destruction, identity, portability, contention) = match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => (
            MemoryAliasing::Unique, MemoryDomain::Inline, MemoryDestruction::Trivial,
            MemoryIdentity::Value, MemoryPortability::Portable, MemoryContention::None,
        ),
        Type::Str => structural_domain(MemoryPortability::WorkerLocal),
        Type::Path => structural_domain(MemoryPortability::LinuxHost),
        Type::Bytes | Type::ByteVector => (
            MemoryAliasing::Unique, MemoryDomain::UniqueStructural, MemoryDestruction::DropGlue,
            MemoryIdentity::Value, MemoryPortability::WorkerLocal,
            MemoryContention::SingleOwner,
        ),
        Type::ByteSlice => borrowed_domain(false),
        Type::ByteSliceMut => borrowed_domain(true),
        Type::Symbol | Type::Fn { .. } | Type::Forall { .. } => static_domain(),
        Type::Resource(_) => (
            MemoryAliasing::External, MemoryDomain::ExternalResource,
            MemoryDestruction::ExternalClose, MemoryIdentity::ExternalResource,
            MemoryPortability::ProcessLocal, MemoryContention::ProviderSerialized,
        ),
        Type::Product(_) => product_domain(fact)?,
        Type::Enum { .. } => aggregate_domain(fact, MemoryPortability::WorkerLocal),
        Type::List(_) if fact.closure.class == MemoryClosureClass::RegionClosed => {
            region_list_domain()
        }
        Type::List(_) => unsupported_domain(MemoryPortability::WorkerLocal),
        Type::Param(_) => (
            MemoryAliasing::StaticShared, MemoryDomain::CallerDestination,
            MemoryDestruction::Trivial, MemoryIdentity::Value, MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
        ),
    };
    let execution_cutover = if fact.closure.class == MemoryClosureClass::Deterministic {
        execution_cutover(ty)
    } else {
        None
    };
    let execution = if execution_cutover.is_some() || domain == MemoryDomain::UnsupportedRuntime {
        MemoryExecution::CutoverRequired
    } else {
        MemoryExecution::Current
    };
    Ok((MemoryMode { multiplicity, aliasing, escape, domain, destruction, identity, portability,
        contention, allocation_failure: allocation_failure(effects) }, execution,
        execution_cutover))
}

fn product_domain(fact: &MemoryTypeFact) -> Result<DomainAxes> {
    match fact.closure.class {
        MemoryClosureClass::Deterministic => {
            Ok(structural_domain(MemoryPortability::WorkerLocal))
        }
        MemoryClosureClass::RegionClosed => Ok(region_list_domain()),
        MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => Err(
            Error::msg("unresolved product reached memory mode derivation"),
        ),
    }
}

fn aggregate_domain(fact: &MemoryTypeFact, portability: MemoryPortability) -> DomainAxes {
    match fact.closure.class {
        MemoryClosureClass::Deterministic => structural_domain(portability),
        MemoryClosureClass::RegionClosed => region_list_domain(),
        MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => {
            unsupported_domain(portability)
        }
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
    )
}

fn region_list_domain() -> DomainAxes {
    (
        MemoryAliasing::RegionShared,
        MemoryDomain::OrdinaryRegion,
        MemoryDestruction::RegionReset,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared,
    )
}

fn unsupported_domain(portability: MemoryPortability) -> DomainAxes {
    (
        MemoryAliasing::UnresolvedShared,
        MemoryDomain::UnsupportedRuntime,
        MemoryDestruction::Unsupported,
        MemoryIdentity::UnsupportedValue,
        portability,
        MemoryContention::UnresolvedShared,
    )
}

fn borrowed_domain(exclusive: bool) -> DomainAxes {
    (if exclusive { MemoryAliasing::BorrowedExclusive } else { MemoryAliasing::BorrowedShared },
        MemoryDomain::BorrowedView, MemoryDestruction::EndBorrow, MemoryIdentity::Value,
        MemoryPortability::WorkerLocal, MemoryContention::SingleOwner)
}

fn static_domain() -> DomainAxes {
    (MemoryAliasing::StaticShared, MemoryDomain::Static, MemoryDestruction::Trivial,
        MemoryIdentity::Value, MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared)
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
