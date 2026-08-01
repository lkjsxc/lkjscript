fn map_mode(value: crate::memory_plan::MemoryAggregateMode) -> MemoryWitnessMode {
    match value {
        crate::memory_plan::MemoryAggregateMode::Copy => MemoryWitnessMode::Copy,
        crate::memory_plan::MemoryAggregateMode::ImmutableValue => MemoryWitnessMode::ImmutableValue,
        crate::memory_plan::MemoryAggregateMode::Affine => MemoryWitnessMode::Affine,
    }
}

fn map_domain(value: crate::memory_plan::MemoryDomain) -> MemoryWitnessDomain {
    use crate::memory_plan::MemoryDomain as Source;
    match value {
        Source::Inline => MemoryWitnessDomain::Inline,
        Source::Static => MemoryWitnessDomain::Static,
        Source::Stack => MemoryWitnessDomain::Stack,
        Source::CallerDestination => MemoryWitnessDomain::CallerDestination,
        Source::UniqueStructural => MemoryWitnessDomain::UniqueStructural,
        Source::OrdinaryRegion => MemoryWitnessDomain::OrdinaryRegion,
        Source::SealedRegion => MemoryWitnessDomain::SealedRegion,
        Source::BorrowedView => MemoryWitnessDomain::BorrowedView,
        Source::ExternalResource => MemoryWitnessDomain::ExternalResource,
        Source::UnsupportedRuntime => MemoryWitnessDomain::Unsupported,
    }
}

fn map_copy(value: crate::memory_plan::MemoryCopySharePlan) -> MemoryWitnessCopy {
    use crate::memory_plan::MemoryCopySharePlan as Source;
    match value {
        Source::TrivialCopy => MemoryWitnessCopy::Trivial,
        Source::StaticIdentity => MemoryWitnessCopy::StaticIdentity,
        Source::StructuralCopy => MemoryWitnessCopy::Structural,
        Source::BorrowShared => MemoryWitnessCopy::BorrowShared,
        Source::BorrowExclusive => MemoryWitnessCopy::BorrowExclusive,
        Source::Move => MemoryWitnessCopy::Move,
        Source::SealedShare => MemoryWitnessCopy::SealedShare,
        Source::RegionHandleCopy => MemoryWitnessCopy::RegionHandle,
        Source::ExternalHandle => MemoryWitnessCopy::ExternalHandle,
        Source::Unsupported => MemoryWitnessCopy::Unsupported,
    }
}

fn witness_drop(facts: &crate::memory_plan::MemoryWitnessFacts) -> MemoryWitnessDrop {
    use crate::memory_plan::MemoryDomain;
    if facts.drop_glue.is_some() {
        MemoryWitnessDrop::Structural
    } else {
        match facts.domain {
            MemoryDomain::OrdinaryRegion | MemoryDomain::SealedRegion => MemoryWitnessDrop::RegionReset,
            MemoryDomain::ExternalResource => MemoryWitnessDrop::External,
            MemoryDomain::UnsupportedRuntime => MemoryWitnessDrop::Unsupported,
            _ => MemoryWitnessDrop::Trivial,
        }
    }
}

fn map_equality(value: crate::memory_plan::MemoryEqualitySupport) -> MemoryWitnessEquality {
    use crate::memory_plan::MemoryEqualitySupport as Source;
    match value {
        Source::Unsupported => MemoryWitnessEquality::Unsupported,
        Source::EqualValue => MemoryWitnessEquality::Value,
        Source::EqualList => MemoryWitnessEquality::List,
        Source::CallerWitnessRequired => MemoryWitnessEquality::Caller,
    }
}

fn map_codec(value: crate::memory_plan::MemoryProcessCodecEligibility) -> MemoryWitnessCodec {
    match value {
        crate::memory_plan::MemoryProcessCodecEligibility::Eligible => MemoryWitnessCodec::Eligible,
        crate::memory_plan::MemoryProcessCodecEligibility::Ineligible => MemoryWitnessCodec::Ineligible,
        crate::memory_plan::MemoryProcessCodecEligibility::CallerWitnessRequired => MemoryWitnessCodec::Caller,
    }
}

fn map_list_element(
    value: crate::memory_plan::MemoryListElementEligibility,
) -> MemoryWitnessListElement {
    use crate::memory_plan::MemoryListElementEligibility as Source;
    match value {
        Source::Copy => MemoryWitnessListElement::Copy,
        Source::ImmutableValue => MemoryWitnessListElement::ImmutableValue,
        Source::UnsupportedAffine => MemoryWitnessListElement::UnsupportedAffine,
        Source::UnsupportedBorrow => MemoryWitnessListElement::UnsupportedBorrow,
        Source::UnsupportedUnresolved => MemoryWitnessListElement::UnsupportedUnresolved,
        Source::CallerWitnessRequired => MemoryWitnessListElement::Caller,
    }
}

fn witness_size(
    ty: &MemoryType,
    size: crate::memory_plan::MemoryDynamicSize,
) -> MemoryWitnessSize {
    if matches!(size, crate::memory_plan::MemoryDynamicSize::CallerWitnessRequired) {
        return MemoryWitnessSize::Caller;
    }
    if matches!(size, crate::memory_plan::MemoryDynamicSize::Dynamic) {
        return MemoryWitnessSize::CheckedDynamic;
    }
    MemoryWitnessSize::Fixed(match ty {
        MemoryType::Unit => 0,
        MemoryType::Bool => 1,
        MemoryType::I64 | MemoryType::F64 => 8,
        _ => 16,
    })
}

fn witness_alignment(ty: &MemoryType) -> u16 {
    match ty {
        MemoryType::Unit | MemoryType::Bool => 1,
        _ => 8,
    }
}

fn map_portability(value: crate::memory_plan::MemoryPortability) -> MemoryWitnessPortability {
    match value {
        crate::memory_plan::MemoryPortability::Portable => MemoryWitnessPortability::Portable,
        crate::memory_plan::MemoryPortability::WorkerLocal => MemoryWitnessPortability::WorkerLocal,
        crate::memory_plan::MemoryPortability::ProcessLocal => MemoryWitnessPortability::ProcessLocal,
        crate::memory_plan::MemoryPortability::LinuxHost => MemoryWitnessPortability::LinuxHost,
    }
}

fn map_contention(value: crate::memory_plan::MemoryContention) -> MemoryWitnessContention {
    use crate::memory_plan::MemoryContention as Source;
    match value {
        Source::None => MemoryWitnessContention::None,
        Source::SingleOwner => MemoryWitnessContention::SingleOwner,
        Source::ImmutableShared => MemoryWitnessContention::ImmutableShared,
        Source::UnresolvedShared => MemoryWitnessContention::UnresolvedShared,
        Source::ProviderSerialized => MemoryWitnessContention::ProviderSerialized,
    }
}
