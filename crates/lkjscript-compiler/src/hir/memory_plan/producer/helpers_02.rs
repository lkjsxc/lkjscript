fn memory_mode(
    ty: &Type,
    effects: u16,
    escape: MemoryEscape,
) -> (MemoryMode, Option<&'static str>, Option<MemoryDropGlueId>) {
    let allocation_failure = allocation_failure(effects);
    let (
        multiplicity,
        aliasing,
        storage,
        destruction,
        identity,
        portability,
        contention,
        family,
        glue,
    ) = match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => (
            MemoryMultiplicity::Copy,
            MemoryAliasing::Unique,
            MemoryStorage::Inline,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::Portable,
            MemoryContention::None,
            None,
            None,
        ),
        Type::Str => legacy_value("string", MemoryMultiplicity::ImmutableValue),
        Type::Buf => (
            MemoryMultiplicity::Copy,
            MemoryAliasing::LegacyTracedShared,
            MemoryStorage::LegacyTraced,
            MemoryDestruction::LegacyTraced,
            MemoryIdentity::LegacyObject,
            MemoryPortability::WorkerLocal,
            MemoryContention::LegacyShared,
            Some("buf"),
            None,
        ),
        Type::Path => (
            MemoryMultiplicity::ImmutableValue,
            MemoryAliasing::LegacyTracedShared,
            MemoryStorage::LegacyTraced,
            MemoryDestruction::LegacyTraced,
            MemoryIdentity::Value,
            MemoryPortability::LinuxHost,
            MemoryContention::ImmutableShared,
            Some("path"),
            None,
        ),
        Type::ByteVector => (
            MemoryMultiplicity::Affine,
            MemoryAliasing::Unique,
            MemoryStorage::LegacyTraced,
            MemoryDestruction::DropGlue,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::SingleOwner,
            Some("buf"),
            Some(MemoryDropGlueId::new(0)),
        ),
        Type::ByteSlice => borrowed(false),
        Type::ByteSliceMut => borrowed(true),
        Type::Symbol => legacy_value("symbol", MemoryMultiplicity::ImmutableValue),
        Type::Resource(kind) => (
            MemoryMultiplicity::Affine,
            MemoryAliasing::External,
            MemoryStorage::ExternalSlot,
            MemoryDestruction::ExternalClose,
            MemoryIdentity::ExternalResource,
            MemoryPortability::ProcessLocal,
            MemoryContention::ProviderSerialized,
            None,
            Some(resource_glue(*kind)),
        ),
        Type::Product(_) => legacy_value("product", MemoryMultiplicity::ImmutableValue),
        Type::Enum { .. } => legacy_value("enum", MemoryMultiplicity::ImmutableValue),
        Type::List(_) => legacy_value("pair", MemoryMultiplicity::ImmutableValue),
        Type::Fn { .. } | Type::Forall { .. } => legacy_value("closure", MemoryMultiplicity::Copy),
        Type::Param(_) => (
            MemoryMultiplicity::ImmutableValue,
            MemoryAliasing::StaticShared,
            MemoryStorage::CallerDestination,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
            None,
            None,
        ),
    };
    (
        MemoryMode {
            multiplicity,
            aliasing,
            escape,
            storage,
            destruction,
            identity,
            portability,
            contention,
            allocation_failure,
        },
        family,
        glue,
    )
}
#[allow(clippy::type_complexity)]
fn legacy_value(
    family: &'static str,
    multiplicity: MemoryMultiplicity,
) -> (
    MemoryMultiplicity,
    MemoryAliasing,
    MemoryStorage,
    MemoryDestruction,
    MemoryIdentity,
    MemoryPortability,
    MemoryContention,
    Option<&'static str>,
    Option<MemoryDropGlueId>,
) {
    (
        multiplicity,
        MemoryAliasing::LegacyTracedShared,
        MemoryStorage::LegacyTraced,
        MemoryDestruction::LegacyTraced,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared,
        Some(family),
        None,
    )
}
#[allow(clippy::type_complexity)]
fn borrowed(
    exclusive: bool,
) -> (
    MemoryMultiplicity,
    MemoryAliasing,
    MemoryStorage,
    MemoryDestruction,
    MemoryIdentity,
    MemoryPortability,
    MemoryContention,
    Option<&'static str>,
    Option<MemoryDropGlueId>,
) {
    (
        MemoryMultiplicity::Borrowed,
        if exclusive {
            MemoryAliasing::BorrowedExclusive
        } else {
            MemoryAliasing::BorrowedShared
        },
        MemoryStorage::BorrowedView,
        MemoryDestruction::EndBorrow,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::SingleOwner,
        None,
        None,
    )
}
