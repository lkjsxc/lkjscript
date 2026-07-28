use lkjscript_core::UniqueStoreStats;

pub(super) fn add_stats(total: &mut UniqueStoreStats, part: UniqueStoreStats) {
    total.allocations = total.allocations.saturating_add(part.allocations);
    total.frees = total.frees.saturating_add(part.frees);
    total.transfers = total.transfers.saturating_add(part.transfers);
    total.live_objects = total.live_objects.saturating_add(part.live_objects);
    total.peak_live_objects = total
        .peak_live_objects
        .saturating_add(part.peak_live_objects);
    total.live_bytes = total.live_bytes.saturating_add(part.live_bytes);
    total.peak_live_bytes = total.peak_live_bytes.saturating_add(part.peak_live_bytes);
    total.reused_slots = total.reused_slots.saturating_add(part.reused_slots);
    total.retired_slots = total.retired_slots.saturating_add(part.retired_slots);
    total.stale_failures = total.stale_failures.saturating_add(part.stale_failures);
    total.wrong_layout_failures = total
        .wrong_layout_failures
        .saturating_add(part.wrong_layout_failures);
    total.allocated_bytes = total.allocated_bytes.saturating_add(part.allocated_bytes);
}
