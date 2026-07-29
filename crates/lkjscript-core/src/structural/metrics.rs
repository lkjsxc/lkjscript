#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralRuntimeMetrics {
    pub domains_created: u64,
    pub domains_released: u64,
    pub slots_reused: u64,
    pub slots_retired: u64,
    pub live_domains: u64,
    pub peak_live_domains: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegionMetrics {
    pub regions_created: u64,
    pub regions_destroyed: u64,
    pub regions_reset: u64,
    pub chunks_created: u64,
    pub objects_allocated: u64,
    pub bytes_allocated: u64,
    pub dependency_edges: u64,
    pub drop_entries: u64,
    pub internal_edges: u64,
    pub release_work: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SealedRegionMetrics {
    pub builders_created: u64,
    pub regions_sealed: u64,
    pub regions_destroyed: u64,
    pub roots_published: u64,
    pub retains: u64,
    pub releases: u64,
    pub weak_upgrades: u64,
    pub rejected_cycles: u64,
    pub dependency_edges: u64,
    pub release_work: u64,
    pub bytes_allocated: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PoolMetrics {
    pub inserts: u64,
    pub removes: u64,
    pub stale_failures: u64,
    pub slots_reused: u64,
    pub slots_retired: u64,
    pub live_slots: u64,
    pub peak_live_slots: u64,
    pub bytes_live: u64,
}
