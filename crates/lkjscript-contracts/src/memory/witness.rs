/// Closed executable capabilities supplied by one hidden static memory witness.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryWitnessOperation {
    Transport,
    Clone,
    Drop,
    Share,
    Compare,
    Encode,
    Decode,
    ListImport,
    ListExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessMode {
    Copy,
    ImmutableValue,
    Affine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessDomain {
    Inline,
    Static,
    Stack,
    CallerDestination,
    UniqueStructural,
    OrdinaryRegion,
    SealedRegion,
    BorrowedView,
    ExternalResource,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessRoot {
    None,
    Structural,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessCopy {
    Trivial,
    StaticIdentity,
    Structural,
    BorrowShared,
    BorrowExclusive,
    Move,
    SealedShare,
    RegionHandle,
    ExternalHandle,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessDrop {
    Trivial,
    Structural,
    RegionReset,
    External,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessEquality {
    Unsupported,
    Value,
    List,
    Caller,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessCodec {
    Eligible,
    Ineligible,
    Caller,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessListElement {
    Copy,
    ImmutableValue,
    UnsupportedAffine,
    UnsupportedBorrow,
    UnsupportedUnresolved,
    Caller,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessSize {
    Fixed(u64),
    CheckedDynamic,
    Caller,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessPortability {
    Portable,
    WorkerLocal,
    ProcessLocal,
    LinuxHost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessContention {
    None,
    SingleOwner,
    ImmutableShared,
    UnresolvedShared,
    ProviderSerialized,
}

/// Runtime-independent facts copied from independently verified HIR authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableMemoryWitnessFacts {
    pub semantic_type: [u8; 32],
    pub semantic_contract: [u8; 32],
    pub mode: MemoryWitnessMode,
    pub domain: MemoryWitnessDomain,
    pub root: MemoryWitnessRoot,
    pub copy: MemoryWitnessCopy,
    pub drop: MemoryWitnessDrop,
    pub equality: MemoryWitnessEquality,
    pub codec: MemoryWitnessCodec,
    pub list_element: MemoryWitnessListElement,
    pub size: MemoryWitnessSize,
    pub alignment: u16,
    pub contains_borrow: bool,
    pub contains_dynamic_owner: bool,
    pub portability: MemoryWitnessPortability,
    pub contention: MemoryWitnessContention,
    pub operations: Vec<MemoryWitnessOperation>,
}
