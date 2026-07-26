#![allow(clippy::unwrap_used)]

use super::*;

const CATEGORY: ResourceCategory = ResourceCategory::SemanticSessionOutputBytes;

fn run() -> BudgetPrefix {
    let mut ledger = BudgetLedger::default();
    for request in 0..128_u64 {
        ledger.rollover_request_segment();
        let mut scope = ledger.scope(BudgetAuthority::SemanticRequest);
        scope
            .reserve(CATEGORY, request + 1, BudgetCause::ProtocolFrame(request))
            .unwrap()
            .commit();
    }
    ledger.prefix()
}

#[test]
fn request_segments_retain_totals_and_only_current_events() {
    let prefix = run();
    assert_eq!(prefix.committed(CATEGORY), (1..=128_u64).sum());
    assert_eq!(prefix.reservation_count(), 1);
    let current = prefix.reservations()[0];
    assert_eq!(current.id.get(), 128);
    assert_eq!(current.amount, 128);
    assert_eq!(current.consumed, 128);
}

#[test]
fn request_segment_prefix_is_deterministic() {
    assert_eq!(run(), run());
}
