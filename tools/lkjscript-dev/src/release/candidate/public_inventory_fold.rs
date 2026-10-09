//! Exact native fold witnesses required before any finalized-byte execution can be accepted.
use super::{DevError, require};
use std::collections::BTreeSet;

const PREFIX: &str = "native_declarations::native_fold::";
const REQUIRED: [&str; 5] = [
    "native_blocked_folds_import_exactly_and_preserve_complete_results_after_source_removal",
    "native_blocked_folds_reject_wrong_contract_without_changing_accepted_consumer",
    "native_blocked_folds_admit_the_complete_argument_before_invoking_a_callback",
    "history::native_blocked_folds_retain_every_nested_prefix_across_blocks_and_source_removal",
    "history::native_blocked_folds_admit_nested_input_and_unused_initial_history_completely",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required native fold behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
