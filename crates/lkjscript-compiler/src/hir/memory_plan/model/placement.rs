#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoryValueRepresentationId([u8; 32]);

impl MemoryValueRepresentationId {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryValueCategory {
    Owner,
    View,
    Destination,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryValueRoute {
    Borrow,
    LastUseMove,
    UniqueReuse,
    DetachedClone,
    SealedShare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryValueFailureCleanup {
    None,
    EndBorrow,
    DisposeUniqueOwner,
    DisposeSealedOwner,
    AbortDestination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryValuePlacement {
    pub expression: MemoryExpressionId,
    pub type_fact: MemoryTypeFactId,
    pub witness: MemoryWitnessId,
    pub use_count: u64,
    pub last_use: bool,
    pub escape: MemoryEscape,
    pub returned: bool,
    pub captured: bool,
    pub process_boundary: bool,
    pub branch_divergence: bool,
    pub independently_live_owners: u64,
    pub independent_owner_demand: bool,
    pub structural_nodes: u64,
    pub payload_bytes: u64,
    pub clone_cost: u64,
    pub dependency_count: u64,
    pub dependency_cost: u64,
    pub release_cost: u64,
    pub representation: MemoryValueRepresentationId,
    pub storage: MemoryDomain,
    pub category: MemoryValueCategory,
    pub route: MemoryValueRoute,
    pub failure_cleanup: MemoryValueFailureCleanup,
}
