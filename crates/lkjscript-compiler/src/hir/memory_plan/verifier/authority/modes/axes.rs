type DomainAxes = (
    MemoryAliasing,
    MemoryDomain,
    MemoryDestruction,
    MemoryIdentity,
    MemoryPortability,
    MemoryContention,
);

fn verified_domain_axes(ty: &Type, fact: &VerifiedExpectedType) -> DomainAxes {
    match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => (
            MemoryAliasing::Unique,
            MemoryDomain::Inline,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::Portable,
            MemoryContention::None,
        ),
        Type::Str => verified_structural_axes(MemoryPortability::WorkerLocal),
        Type::Path => verified_structural_axes(MemoryPortability::LinuxHost),
        Type::Product(_) | Type::Enum { .. }
            if fact.derived.closure.class == MemoryClosureClass::Deterministic =>
        {
            verified_structural_axes(MemoryPortability::WorkerLocal)
        }
        Type::Product(_) | Type::Enum { .. } | Type::List(_) => (
            MemoryAliasing::LegacyTracedShared,
            MemoryDomain::RegisteredLegacyTraced,
            MemoryDestruction::LegacyTraced,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
        ),
        Type::Bytes | Type::ByteVector => (
            MemoryAliasing::Unique,
            MemoryDomain::UniqueStructural,
            MemoryDestruction::DropGlue,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::SingleOwner,
        ),
        Type::ByteSlice => borrowed_axes(false),
        Type::ByteSliceMut => borrowed_axes(true),
        Type::Symbol | Type::Fn { .. } | Type::Forall { .. } => (
            MemoryAliasing::StaticShared,
            MemoryDomain::Static,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
        ),
        Type::Resource(_) => (
            MemoryAliasing::External,
            MemoryDomain::ExternalResource,
            MemoryDestruction::ExternalClose,
            MemoryIdentity::ExternalResource,
            MemoryPortability::ProcessLocal,
            MemoryContention::ProviderSerialized,
        ),
        Type::Param(_) => (
            MemoryAliasing::StaticShared,
            MemoryDomain::CallerDestination,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
        ),
    }
}

fn verified_structural_axes(portability: MemoryPortability) -> DomainAxes {
    (
        MemoryAliasing::Unique,
        MemoryDomain::UniqueStructural,
        MemoryDestruction::DropGlue,
        MemoryIdentity::Value,
        portability,
        MemoryContention::SingleOwner,
    )
}

fn borrowed_axes(exclusive: bool) -> DomainAxes {
    (
        if exclusive {
            MemoryAliasing::BorrowedExclusive
        } else {
            MemoryAliasing::BorrowedShared
        },
        MemoryDomain::BorrowedView,
        MemoryDestruction::EndBorrow,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::SingleOwner,
    )
}
