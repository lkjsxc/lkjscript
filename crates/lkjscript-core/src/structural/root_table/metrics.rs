#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralRootTableStats {
    pub roots_published: u64,
    pub roots_moved: u64,
    pub roots_dropped: u64,
    pub roots_released: u64,
    pub root_slots_reused: u64,
    pub root_slots_retired: u64,
    pub loans_started: u64,
    pub loans_ended: u64,
    pub loan_slots_reused: u64,
    pub loan_slots_retired: u64,
    pub live_roots: u64,
    pub peak_live_roots: u64,
    pub live_loans: u64,
    pub peak_live_loans: u64,
}
