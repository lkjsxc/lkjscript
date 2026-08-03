use super::*;

unit_enum!(MemoryValueCategory {
    Owner = 0,
    View = 1,
    Destination = 2,
});
unit_enum!(MemoryValueRoute {
    Borrow = 0,
    LastUseMove = 1,
    UniqueReuse = 2,
    DetachedClone = 3,
    SealedShare = 4,
});
unit_enum!(MemoryValueFailureCleanup {
    None = 0,
    EndBorrow = 1,
    DisposeUniqueOwner = 2,
    DisposeSealedOwner = 3,
    AbortDestination = 4,
});

impl Canonical for MemoryValueRepresentationId {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.bytes(&self.as_bytes())
    }
}

canonical_struct!(MemoryValuePlacement {
    expression,
    type_fact,
    witness,
    use_count,
    last_use,
    escape,
    returned,
    captured,
    process_boundary,
    branch_divergence,
    independently_live_owners,
    independent_owner_demand,
    structural_nodes,
    payload_bytes,
    clone_cost,
    dependency_count,
    dependency_cost,
    release_cost,
    representation,
    storage,
    category,
    route,
    failure_cleanup,
});
