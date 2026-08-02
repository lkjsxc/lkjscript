fn verified_witness_capabilities(
    ty: &Type,
    derived: &VerifiedDerived,
) -> lkjscript_contracts::MemoryWitnessCapabilities {
    let domain = verified_witness_domain(ty, derived);
    let codec = verified_witness_process_codec(ty, derived);
    let list = verified_witness_list_element(ty, derived);
    let equality = verified_witness_equality(ty);
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
        sealed_region: immutable && codec == MemoryProcessCodecEligibility::Eligible,
        borrow: !matches!(
            domain,
            MemoryDomain::UnsupportedRuntime | MemoryDomain::ExternalResource
        ),
        process_codec: codec == MemoryProcessCodecEligibility::Eligible,
        list_element: matches!(
            list,
            MemoryListElementEligibility::Copy | MemoryListElementEligibility::ImmutableValue
        ),
        equality: !matches!(equality, MemoryEqualitySupport::Unsupported),
    }
}
