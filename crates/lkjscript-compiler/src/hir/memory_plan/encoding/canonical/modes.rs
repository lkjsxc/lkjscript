use super::*;

unit_enum!(MemoryMultiplicity {
    Copy = 0,
    ImmutableValue = 1,
    Affine = 2,
    Borrowed = 3,
});
unit_enum!(MemoryAliasing {
    Unique = 0,
    BorrowedShared = 1,
    BorrowedExclusive = 2,
    StaticShared = 3,
    RegionShared = 4,
    UnresolvedShared = 5,
    External = 6,
});
unit_enum!(MemoryEscape {
    Local = 0,
    Caller = 1,
    Returned = 2,
    Captured = 3,
    Runtime = 4,
    Static = 5,
});
unit_enum!(MemoryDestruction {
    Trivial = 0,
    EndBorrow = 1,
    DropGlue = 2,
    ExternalClose = 3,
    RegionReset = 4,
    Unsupported = 5,
});
unit_enum!(MemoryIdentity { Value = 0, ExternalResource = 1, UnsupportedValue = 2 });
unit_enum!(MemoryPortability { Portable = 0, WorkerLocal = 1, ProcessLocal = 2, LinuxHost = 3 });
unit_enum!(MemoryContention {
    None = 0,
    SingleOwner = 1,
    ImmutableShared = 2,
    UnresolvedShared = 3,
    ProviderSerialized = 4,
});
unit_enum!(MemoryAllocationFailure {
    Impossible = 0,
    Trap = 1,
    StructuredOutcome = 2,
    TrapOrOutcome = 3,
});
canonical_struct!(MemoryMode {
    multiplicity,
    aliasing,
    escape,
    domain,
    destruction,
    identity,
    portability,
    contention,
    allocation_failure,
});
