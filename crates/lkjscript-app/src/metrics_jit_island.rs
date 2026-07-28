use lkjscript_jit::JitStats;

pub(super) fn render(stats: &JitStats) -> String {
    let resources = stats.native_resources;
    let unique = stats.native_unique;
    format!(
        concat!(
            "{{\"collector_runtime_invocations\":{},\"resource_runtime_calls\":{},",
            "\"unique_runtime_calls\":{},\"native_resources\":{{",
            "\"reservations\":{},\"borrowed_installs\":{},\"borrowed_reuses\":{},",
            "\"borrowed_removals\":{},\"ordinary_obligations\":{},",
            "\"borrowed_obligations\":{},\"emergency_obligations\":{},",
            "\"teardown_failures\":{}}},\"native_unique\":{{",
            "\"allocations\":{},\"moves\":{},\"shared_borrows\":{},",
            "\"exclusive_borrows\":{},\"length_reads\":{},\"byte_reads\":{},",
            "\"byte_writes\":{},\"loan_ends\":{},\"drops\":{},",
            "\"transfers\":{},\"cleanup_attempts\":{},\"cleanup_releases\":{},",
            "\"stale_or_forged_failures\":{},\"live_owners\":{},",
            "\"live_loans\":{},\"release_backlog\":{},\"teardown_failures\":{}}}}}"
        ),
        stats.collector_runtime_invocations,
        stats.resource_runtime_calls,
        stats.unique_runtime_calls,
        resources.reservations,
        resources.borrowed_installs,
        resources.borrowed_reuses,
        resources.borrowed_removals,
        resources.ordinary_obligations,
        resources.borrowed_obligations,
        resources.emergency_obligations,
        resources.teardown_failures,
        unique.allocations,
        unique.moves,
        unique.shared_borrows,
        unique.exclusive_borrows,
        unique.length_reads,
        unique.byte_reads,
        unique.byte_writes,
        unique.loan_ends,
        unique.drops,
        unique.transfers,
        unique.cleanup_attempts,
        unique.cleanup_releases,
        unique.stale_or_forged_failures,
        unique.live_owners,
        unique.live_loans,
        unique.release_backlog,
        unique.teardown_failures,
    )
}
