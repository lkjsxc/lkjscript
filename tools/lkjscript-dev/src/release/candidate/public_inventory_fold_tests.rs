//! Native consumer behavior cannot be replaced with a host-only oracle or partial family.
use super::*;

const PREFIX: &str = "native_declarations::native_fold::";
const CASES: [&str; 5] = [
    "native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal",
    "native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer",
    "native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback",
    "history::native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal",
    "history::native_blocked_folds_admit_nested_input_and_unused_initial_history_completely",
];

fn listing() -> String {
    tests::listing().replace("unrelated_test: test\n", "")
}

fn success(inventory: &Inventory) -> String {
    let mut text = format!("\nrunning {} tests\n", inventory.selected.len());
    for case in &inventory.selected {
        text.push_str(&format!("test {case} ... ok\n"));
    }
    text.push_str(&format!("\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; {} filtered out; finished in 1.0s\n", inventory.selected.len(), inventory.all.len() - inventory.selected.len()));
    text
}

#[test]
fn native_fold_inventory_requires_every_behavior_witness_not_just_an_oracle() {
    for omitted in CASES {
        let text = listing().replace(&format!("{PREFIX}{omitted}: test\n"), "");
        assert!(inventory(&text).is_err(), "missing required {omitted}");
    }
    let mut oracle = listing();
    for suffix in CASES {
        oracle = oracle.replace(&format!("{PREFIX}{suffix}: test\n"), "");
    }
    oracle.push_str(&format!(
        "{PREFIX}cases::positional_oracle_is_order_sensitive: test\n"
    ));
    assert!(inventory(&oracle).is_err());
}

#[test]
fn native_fold_inventory_selects_future_cases_and_checks_each_complete_terminal() {
    let listing = format!(
        "{}{PREFIX}future_case: test\n{PREFIX}cases::oracle: test\nunrelated: test\n",
        listing()
    );
    let inventory = inventory(&listing).unwrap();
    assert_eq!(inventory.selected.len(), 29);
    assert_eq!(inventory.all.len(), 30);
    assert!(!inventory.selected.contains("unrelated"));
    for suffix in CASES.into_iter().chain(["future_case", "cases::oracle"]) {
        assert!(inventory.selected.contains(&format!("{PREFIX}{suffix}")));
    }
    let complete = success(&inventory);
    passed(&complete, &inventory).unwrap();
    for suffix in CASES.into_iter().chain(["future_case", "cases::oracle"]) {
        let record = format!("test {PREFIX}{suffix} ... ok");
        for broken in [
            complete.replace(&format!("{record}\n"), ""),
            complete.replace(&record, &record.replace(" ... ok", " ... ignored")),
            complete.replace(&record, &record.replace(" ... ok", " ... FAILED")),
            complete.clone() + &record + "\n",
        ] {
            assert!(passed(&broken, &inventory).is_err());
        }
    }
}

#[test]
fn native_fold_inventory_rejects_namespace_and_function_substitution() {
    for replacement in [
        "native_fold::",
        "native_declarations::prefix_native_fold::",
        "native_declarations::native_fold_extra::",
    ] {
        assert!(inventory(&listing().replace(PREFIX, replacement)).is_err());
    }
    for suffix in CASES {
        assert!(inventory(&listing().replace(suffix, &format!("prefix_{suffix}"))).is_err());
    }
}
