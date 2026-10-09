use super::*;

pub(super) fn inputs() -> Vec<Vec<i64>> {
    [
        0, 1, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 127, 128, 129, 1024,
    ]
    .into_iter()
    .map(|n| (0..n).map(|i| (i * 17 + 3) % 29 - 14).collect())
    .collect()
}

pub(super) fn expected(items: &[i64], initial_count: i64, initial_sum: i64) -> Value {
    let count = initial_count + i64::try_from(items.len()).unwrap();
    // Independent positional dot product, not the library's loop or standard fold.
    let sum = initial_sum
        + items
            .iter()
            .enumerate()
            .map(|(index, item)| (initial_count + i64::try_from(index).unwrap() + 1) * item)
            .sum::<i64>();
    json!({"count":count,"sum":sum})
}

pub(super) fn joined(execution: &CompactRecord) {
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    for key in [
        "live_call_frames_after",
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        assert_eq!(observation[key], 0, "unreleased {key}");
    }
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["admission_stopped"], true);
    assert_eq!(cleanup["remaining_tasks"], 0);
    assert_eq!(cleanup["cleanup_failures"], json!([]));
    let executor: Value =
        serde_json::from_str(compact_field(execution, "executor-observation")).unwrap();
    assert_eq!(executor["dispatch_open"], false);
    assert_eq!(executor["active_dispatches"], 0);
    assert_eq!(executor["remaining_workers"], 0);
}

pub(super) fn failed_join(diagnostic: &CompactRecord) {
    let notes: Vec<String> = serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
    assert!(notes.iter().any(|note| note
        == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
    assert!(notes.iter().any(|note| note == "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers=0"));
}

#[test]
fn positional_oracle_is_order_sensitive() {
    assert_eq!(expected(&[7, 8, 9], 3, 11), json!({"count":6,"sum":133}));
    assert_ne!(expected(&[7, 8, 9], 3, 11), expected(&[9, 8, 7], 3, 11));
    assert_eq!(expected(&[], 3, 11), json!({"count":3,"sum":11}));
}
