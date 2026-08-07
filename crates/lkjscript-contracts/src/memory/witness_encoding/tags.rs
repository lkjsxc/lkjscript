use super::super::witness::*;

pub(super) const fn mode(value: MemoryWitnessMode) -> u8 {
    match value {
        MemoryWitnessMode::Copy => 0,
        MemoryWitnessMode::ImmutableValue => 1,
        MemoryWitnessMode::Affine => 2,
    }
}

pub(super) const fn domain(value: MemoryWitnessDomain) -> u8 {
    match value {
        MemoryWitnessDomain::Inline => 0,
        MemoryWitnessDomain::Static => 1,
        MemoryWitnessDomain::Stack => 2,
        MemoryWitnessDomain::CallerDestination => 3,
        MemoryWitnessDomain::UniqueStructural => 4,
        MemoryWitnessDomain::OrdinaryRegion => 5,
        MemoryWitnessDomain::SealedRegion => 6,
        MemoryWitnessDomain::BorrowedView => 7,
        MemoryWitnessDomain::ExternalResource => 8,
        MemoryWitnessDomain::Unsupported => 9,
    }
}

pub(super) const fn root(value: MemoryWitnessRoot) -> u8 {
    match value {
        MemoryWitnessRoot::None => 0,
        MemoryWitnessRoot::Structural => 1,
    }
}

pub(super) const fn copy(value: MemoryWitnessCopy) -> u8 {
    match value {
        MemoryWitnessCopy::Trivial => 0,
        MemoryWitnessCopy::StaticIdentity => 1,
        MemoryWitnessCopy::Structural => 2,
        MemoryWitnessCopy::BorrowShared => 3,
        MemoryWitnessCopy::BorrowExclusive => 4,
        MemoryWitnessCopy::Move => 5,
        MemoryWitnessCopy::SealedShare => 6,
        MemoryWitnessCopy::RegionHandle => 7,
        MemoryWitnessCopy::ExternalHandle => 8,
        MemoryWitnessCopy::Unsupported => 9,
    }
}

pub(super) const fn drop_route(value: MemoryWitnessDrop) -> u8 {
    match value {
        MemoryWitnessDrop::Trivial => 0,
        MemoryWitnessDrop::Structural => 1,
        MemoryWitnessDrop::RegionReset => 2,
        MemoryWitnessDrop::External => 3,
        MemoryWitnessDrop::Unsupported => 4,
    }
}

pub(super) const fn equality(value: MemoryWitnessEquality) -> u8 {
    match value {
        MemoryWitnessEquality::Unsupported => 0,
        MemoryWitnessEquality::Value => 1,
        MemoryWitnessEquality::List => 2,
        MemoryWitnessEquality::Caller => 3,
    }
}

pub(super) const fn snapshot(value: MemoryWitnessSnapshot) -> u8 {
    match value {
        MemoryWitnessSnapshot::Eligible => 0,
        MemoryWitnessSnapshot::Ineligible => 1,
        MemoryWitnessSnapshot::Caller => 2,
    }
}

pub(super) const fn list_element(value: MemoryWitnessListElement) -> u8 {
    match value {
        MemoryWitnessListElement::Copy => 0,
        MemoryWitnessListElement::ImmutableValue => 1,
        MemoryWitnessListElement::UnsupportedAffine => 2,
        MemoryWitnessListElement::UnsupportedBorrow => 3,
        MemoryWitnessListElement::UnsupportedUnresolved => 4,
        MemoryWitnessListElement::Caller => 5,
    }
}

pub(super) const fn portability(value: MemoryWitnessPortability) -> u8 {
    match value {
        MemoryWitnessPortability::Portable => 0,
        MemoryWitnessPortability::WorkerLocal => 1,
        MemoryWitnessPortability::ProcessLocal => 2,
        MemoryWitnessPortability::LinuxHost => 3,
    }
}

pub(super) const fn contention(value: MemoryWitnessContention) -> u8 {
    match value {
        MemoryWitnessContention::None => 0,
        MemoryWitnessContention::SingleOwner => 1,
        MemoryWitnessContention::ImmutableShared => 2,
        MemoryWitnessContention::UnresolvedShared => 3,
        MemoryWitnessContention::ProviderSerialized => 4,
    }
}

pub(super) const fn operation_tag(value: MemoryWitnessOperation) -> u8 {
    match value {
        MemoryWitnessOperation::Transport => 0,
        MemoryWitnessOperation::Clone => 1,
        MemoryWitnessOperation::Drop => 2,
        MemoryWitnessOperation::Share => 3,
        MemoryWitnessOperation::Compare => 4,
        MemoryWitnessOperation::SnapshotExport => 5,
        MemoryWitnessOperation::SnapshotImport => 6,
        MemoryWitnessOperation::ListImport => 7,
        MemoryWitnessOperation::ListExport => 8,
        MemoryWitnessOperation::IndependentOwner => 9,
        MemoryWitnessOperation::Dispose => 10,
    }
}
