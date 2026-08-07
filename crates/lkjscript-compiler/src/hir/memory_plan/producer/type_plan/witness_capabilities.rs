fn witness_capabilities(
    ty: &Type,
    derived: &DerivedType,
) -> lkjscript_contracts::MemoryWitnessCapabilities {
    let domain = witness_domain(ty, derived);
    let snapshot = witness_semantic_snapshot(ty, derived);
    let list = witness_list_element(ty, derived);
    let equality = witness_equality(ty);
    let immutable = derived.mode != MemoryAggregateMode::Affine
        && (derived.mode == MemoryAggregateMode::ImmutableValue
            || derived.contains_dynamic_owner)
        && matches!(
            derived.closure.class,
            MemoryClosureClass::Deterministic | MemoryClosureClass::RegionClosed
        )
        && !derived.contains_borrow;
    lkjscript_contracts::MemoryWitnessCapabilities {
        inline: domain == MemoryDomain::Inline,
        static_value: domain == MemoryDomain::Static,
        unique: matches!(
            domain,
            MemoryDomain::UniqueStructural
                | MemoryDomain::OrdinaryRegion
                | MemoryDomain::SealedRegion
        ),
        ordinary_region: immutable,
        sealed_region: immutable && snapshot == MemorySemanticSnapshotEligibility::Eligible,
        borrow: !matches!(
            domain,
            MemoryDomain::UnsupportedRuntime | MemoryDomain::ExternalResource
        ),
        semantic_snapshot: snapshot == MemorySemanticSnapshotEligibility::Eligible,
        list_element: matches!(
            list,
            MemoryListElementEligibility::Copy | MemoryListElementEligibility::ImmutableValue
        ),
        equality: !matches!(equality, MemoryEqualitySupport::Unsupported),
    }
}
