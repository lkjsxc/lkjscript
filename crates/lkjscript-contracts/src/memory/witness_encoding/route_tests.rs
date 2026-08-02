#[test]
fn capability_and_selected_routes_are_independently_compatible() {
    let mut ordinary = facts();
    ordinary.operations = required_memory_witness_operations(&ordinary);
    assert!(memory_witness_routes_are_compatible(&ordinary));

    let mut move_only = ordinary.clone();
    move_only.copy = MemoryWitnessCopy::Move;
    move_only.operations = required_memory_witness_operations(&move_only);
    assert!(!move_only
        .operations
        .contains(&MemoryWitnessOperation::Clone));
    assert!(memory_witness_routes_are_compatible(&move_only));

    let mut mode_crossing = ordinary.clone();
    mode_crossing.mode = MemoryWitnessMode::Copy;
    mode_crossing.operations = required_memory_witness_operations(&mode_crossing);
    assert!(memory_witness_routes_are_compatible(&mode_crossing));
    mode_crossing.contains_dynamic_owner = false;
    mode_crossing.operations = required_memory_witness_operations(&mode_crossing);
    assert!(!memory_witness_routes_are_compatible(&mode_crossing));

    let mut sealed = ordinary.clone();
    sealed.domain = MemoryWitnessDomain::SealedRegion;
    sealed.copy = MemoryWitnessCopy::SealedShare;
    sealed.contention = MemoryWitnessContention::ImmutableShared;
    sealed.operations = required_memory_witness_operations(&sealed);
    assert!(memory_witness_routes_are_compatible(&sealed));

    sealed.capabilities.sealed_region = false;
    sealed.operations = required_memory_witness_operations(&sealed);
    assert!(!memory_witness_routes_are_compatible(&sealed));
}
