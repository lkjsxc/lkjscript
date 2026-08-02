fn map_mode(value: super::MemoryAggregateMode) -> MemoryWitnessMode {
    match value {
        super::MemoryAggregateMode::Copy => MemoryWitnessMode::Copy,
        super::MemoryAggregateMode::ImmutableValue => MemoryWitnessMode::ImmutableValue,
        super::MemoryAggregateMode::Affine => MemoryWitnessMode::Affine,
    }
}

fn map_domain(value: super::MemoryDomain) -> MemoryWitnessDomain {
    match value {
        super::MemoryDomain::Inline => MemoryWitnessDomain::Inline,
        super::MemoryDomain::Static => MemoryWitnessDomain::Static,
        super::MemoryDomain::Stack => MemoryWitnessDomain::Stack,
        super::MemoryDomain::CallerDestination => MemoryWitnessDomain::CallerDestination,
        super::MemoryDomain::UniqueStructural => MemoryWitnessDomain::UniqueStructural,
        super::MemoryDomain::OrdinaryRegion => MemoryWitnessDomain::OrdinaryRegion,
        super::MemoryDomain::SealedRegion => MemoryWitnessDomain::SealedRegion,
        super::MemoryDomain::BorrowedView => MemoryWitnessDomain::BorrowedView,
        super::MemoryDomain::ExternalResource => MemoryWitnessDomain::ExternalResource,
        super::MemoryDomain::UnsupportedRuntime => MemoryWitnessDomain::Unsupported,
    }
}

fn map_copy(value: super::MemoryCopySharePlan) -> MemoryWitnessCopy {
    match value {
        super::MemoryCopySharePlan::TrivialCopy => MemoryWitnessCopy::Trivial,
        super::MemoryCopySharePlan::StaticIdentity => MemoryWitnessCopy::StaticIdentity,
        super::MemoryCopySharePlan::StructuralCopy => MemoryWitnessCopy::Structural,
        super::MemoryCopySharePlan::BorrowShared => MemoryWitnessCopy::BorrowShared,
        super::MemoryCopySharePlan::BorrowExclusive => MemoryWitnessCopy::BorrowExclusive,
        super::MemoryCopySharePlan::Move => MemoryWitnessCopy::Move,
        super::MemoryCopySharePlan::SealedShare => MemoryWitnessCopy::SealedShare,
        super::MemoryCopySharePlan::RegionHandleCopy => MemoryWitnessCopy::RegionHandle,
        super::MemoryCopySharePlan::ExternalHandle => MemoryWitnessCopy::ExternalHandle,
        super::MemoryCopySharePlan::Unsupported => MemoryWitnessCopy::Unsupported,
    }
}

fn witness_drop(facts: &MemoryWitnessFacts) -> MemoryWitnessDrop {
    if facts.drop_glue.is_some() {
        MemoryWitnessDrop::Structural
    } else {
        match facts.domain {
            super::MemoryDomain::OrdinaryRegion | super::MemoryDomain::SealedRegion => {
                MemoryWitnessDrop::RegionReset
            }
            super::MemoryDomain::ExternalResource => MemoryWitnessDrop::External,
            super::MemoryDomain::UnsupportedRuntime => MemoryWitnessDrop::Unsupported,
            _ => MemoryWitnessDrop::Trivial,
        }
    }
}

fn map_equality(value: super::MemoryEqualitySupport) -> MemoryWitnessEquality {
    match value {
        super::MemoryEqualitySupport::Unsupported => MemoryWitnessEquality::Unsupported,
        super::MemoryEqualitySupport::EqualValue => MemoryWitnessEquality::Value,
        super::MemoryEqualitySupport::EqualList => MemoryWitnessEquality::List,
        super::MemoryEqualitySupport::CallerWitnessRequired => MemoryWitnessEquality::Caller,
    }
}

fn map_codec(value: super::MemoryProcessCodecEligibility) -> MemoryWitnessCodec {
    match value {
        super::MemoryProcessCodecEligibility::Eligible => MemoryWitnessCodec::Eligible,
        super::MemoryProcessCodecEligibility::Ineligible => MemoryWitnessCodec::Ineligible,
        super::MemoryProcessCodecEligibility::CallerWitnessRequired => MemoryWitnessCodec::Caller,
    }
}

fn map_list_element(value: super::MemoryListElementEligibility) -> MemoryWitnessListElement {
    match value {
        super::MemoryListElementEligibility::Copy => MemoryWitnessListElement::Copy,
        super::MemoryListElementEligibility::ImmutableValue => {
            MemoryWitnessListElement::ImmutableValue
        }
        super::MemoryListElementEligibility::UnsupportedAffine => {
            MemoryWitnessListElement::UnsupportedAffine
        }
        super::MemoryListElementEligibility::UnsupportedBorrow => {
            MemoryWitnessListElement::UnsupportedBorrow
        }
        super::MemoryListElementEligibility::UnsupportedUnresolved => {
            MemoryWitnessListElement::UnsupportedUnresolved
        }
        super::MemoryListElementEligibility::CallerWitnessRequired => {
            MemoryWitnessListElement::Caller
        }
    }
}

fn witness_size(ty: &MemoryType, size: super::MemoryDynamicSize) -> MemoryWitnessSize {
    if matches!(size, super::MemoryDynamicSize::CallerWitnessRequired) {
        return MemoryWitnessSize::Caller;
    }
    if matches!(size, super::MemoryDynamicSize::Dynamic) {
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

fn map_portability(value: super::MemoryPortability) -> MemoryWitnessPortability {
    match value {
        super::MemoryPortability::Portable => MemoryWitnessPortability::Portable,
        super::MemoryPortability::WorkerLocal => MemoryWitnessPortability::WorkerLocal,
        super::MemoryPortability::ProcessLocal => MemoryWitnessPortability::ProcessLocal,
        super::MemoryPortability::LinuxHost => MemoryWitnessPortability::LinuxHost,
    }
}

fn map_contention(value: super::MemoryContention) -> MemoryWitnessContention {
    match value {
        super::MemoryContention::None => MemoryWitnessContention::None,
        super::MemoryContention::SingleOwner => MemoryWitnessContention::SingleOwner,
        super::MemoryContention::ImmutableShared => MemoryWitnessContention::ImmutableShared,
        super::MemoryContention::UnresolvedShared => MemoryWitnessContention::UnresolvedShared,
        super::MemoryContention::ProviderSerialized => MemoryWitnessContention::ProviderSerialized,
    }
}
