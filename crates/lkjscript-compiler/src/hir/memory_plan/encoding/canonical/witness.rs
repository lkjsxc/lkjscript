use super::*;

unit_enum!(MemoryWitnessRequirement { Concrete = 0, SpecializationRequired = 1 });
unit_enum!(MemoryEqualitySupport {
    Unsupported = 0,
    EqualValue = 1,
    EqualList = 2,
    CallerWitnessRequired = 3,
});
unit_enum!(MemoryProcessCodecEligibility {
    Eligible = 0,
    Ineligible = 1,
    CallerWitnessRequired = 2,
});
unit_enum!(MemoryListElementEligibility {
    Copy = 0,
    ImmutableValue = 1,
    UnsupportedAffine = 2,
    UnsupportedBorrow = 3,
    UnsupportedUnresolved = 4,
    CallerWitnessRequired = 5,
});
unit_enum!(MemoryDynamicSize { Fixed = 0, Dynamic = 1, CallerWitnessRequired = 2 });
unit_enum!(MemoryListStorageKind { SegmentedSessionRegion = 0 });
canonical_struct!(MemoryListWitness {
    element,
    selected,
    eligibility,
    storage,
    segment_capacity,
});
canonical_struct!(MemoryWitnessDropAction { path, glue });
canonical_struct!(MemoryWitnessDropBranch {
    active_variant,
    actions
});
canonical_struct!(MemoryWitnessFacts {
    ty,
    semantic_contract,
    requirement,
    mode,
    closure,
    root_projection,
    domain,
    copy_share,
    drop_glue,
    drop_path,
    equality,
    process_codec,
    list_element,
    list,
    dynamic_size,
    contains_borrow,
    contains_dynamic_owner,
    portability,
    contention,
});
canonical_struct!(MemoryWitness { id, facts });
