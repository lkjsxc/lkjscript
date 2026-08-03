use super::witness::*;

pub fn required_memory_witness_operations(
    facts: &ExecutableMemoryWitnessFacts,
) -> Vec<MemoryWitnessOperation> {
    let mut operations = vec![MemoryWitnessOperation::Transport];
    if matches!(
        facts.copy,
        MemoryWitnessCopy::Trivial
            | MemoryWitnessCopy::StaticIdentity
            | MemoryWitnessCopy::Structural
            | MemoryWitnessCopy::BorrowShared
            | MemoryWitnessCopy::SealedShare
            | MemoryWitnessCopy::RegionHandle
    ) {
        operations.extend([
            MemoryWitnessOperation::Clone,
            MemoryWitnessOperation::IndependentOwner,
        ]);
    }
    operations.push(MemoryWitnessOperation::Dispose);
    if !matches!(
        facts.drop,
        MemoryWitnessDrop::Trivial | MemoryWitnessDrop::Unsupported
    ) {
        operations.push(MemoryWitnessOperation::Drop);
    }
    if facts.copy == MemoryWitnessCopy::SealedShare {
        operations.push(MemoryWitnessOperation::Share);
    }
    if facts.equality != MemoryWitnessEquality::Unsupported {
        operations.push(MemoryWitnessOperation::Compare);
    }
    if facts.codec == MemoryWitnessCodec::Eligible {
        operations.extend([
            MemoryWitnessOperation::Encode,
            MemoryWitnessOperation::Decode,
        ]);
    }
    if matches!(
        facts.list_element,
        MemoryWitnessListElement::Copy | MemoryWitnessListElement::ImmutableValue
    ) {
        operations.extend([
            MemoryWitnessOperation::ListImport,
            MemoryWitnessOperation::ListExport,
        ]);
    }
    operations.sort_unstable();
    operations
}

pub fn memory_witness_routes_are_compatible(facts: &ExecutableMemoryWitnessFacts) -> bool {
    let capabilities = facts.capabilities;
    if capabilities.process_codec != (facts.codec == MemoryWitnessCodec::Eligible)
        || capabilities.list_element
            != matches!(
                facts.list_element,
                MemoryWitnessListElement::Copy | MemoryWitnessListElement::ImmutableValue
            )
        || capabilities.equality != (facts.equality != MemoryWitnessEquality::Unsupported)
    {
        return false;
    }
    let placement_permitted = match facts.domain {
        MemoryWitnessDomain::Inline => capabilities.inline,
        MemoryWitnessDomain::Static => capabilities.static_value,
        MemoryWitnessDomain::UniqueStructural => capabilities.unique,
        MemoryWitnessDomain::OrdinaryRegion => capabilities.ordinary_region,
        MemoryWitnessDomain::SealedRegion => capabilities.sealed_region,
        MemoryWitnessDomain::BorrowedView => capabilities.borrow,
        MemoryWitnessDomain::Stack
        | MemoryWitnessDomain::CallerDestination
        | MemoryWitnessDomain::ExternalResource
        | MemoryWitnessDomain::Unsupported => true,
    };
    if !placement_permitted {
        return false;
    }
    if capabilities.sealed_region
        && (facts.mode == MemoryWitnessMode::Affine
            || (facts.mode != MemoryWitnessMode::ImmutableValue && !facts.contains_dynamic_owner)
            || facts.contains_borrow
            || !capabilities.process_codec)
    {
        return false;
    }
    let sealed_selected = facts.domain == MemoryWitnessDomain::SealedRegion;
    if sealed_selected
        != (facts.copy == MemoryWitnessCopy::SealedShare
            && facts.drop == MemoryWitnessDrop::RegionReset
            && facts.contention == MemoryWitnessContention::ImmutableShared)
    {
        return false;
    }
    required_memory_witness_operations(facts) == facts.operations
}
