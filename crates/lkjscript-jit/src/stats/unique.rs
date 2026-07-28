#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeUniqueStats {
    pub allocations: u64,
    pub moves: u64,
    pub shared_borrows: u64,
    pub exclusive_borrows: u64,
    pub length_reads: u64,
    pub byte_reads: u64,
    pub byte_writes: u64,
    pub loan_ends: u64,
    pub drops: u64,
    pub transfers: u64,
    pub cleanup_attempts: u64,
    pub cleanup_releases: u64,
    pub stale_or_forged_failures: u64,
    pub live_owners: u64,
    pub live_loans: u64,
    pub release_backlog: u64,
    pub teardown_failures: u64,
}

impl NativeUniqueStats {
    pub(crate) fn add(&mut self, other: Self) {
        self.allocations = self.allocations.saturating_add(other.allocations);
        self.moves = self.moves.saturating_add(other.moves);
        self.shared_borrows = self.shared_borrows.saturating_add(other.shared_borrows);
        self.exclusive_borrows = self
            .exclusive_borrows
            .saturating_add(other.exclusive_borrows);
        self.length_reads = self.length_reads.saturating_add(other.length_reads);
        self.byte_reads = self.byte_reads.saturating_add(other.byte_reads);
        self.byte_writes = self.byte_writes.saturating_add(other.byte_writes);
        self.loan_ends = self.loan_ends.saturating_add(other.loan_ends);
        self.drops = self.drops.saturating_add(other.drops);
        self.transfers = self.transfers.saturating_add(other.transfers);
        self.cleanup_attempts = self.cleanup_attempts.saturating_add(other.cleanup_attempts);
        self.cleanup_releases = self.cleanup_releases.saturating_add(other.cleanup_releases);
        self.stale_or_forged_failures = self
            .stale_or_forged_failures
            .saturating_add(other.stale_or_forged_failures);
        self.live_owners = other.live_owners;
        self.live_loans = other.live_loans;
        self.release_backlog = other.release_backlog;
        self.teardown_failures = self
            .teardown_failures
            .saturating_add(other.teardown_failures);
    }
}
