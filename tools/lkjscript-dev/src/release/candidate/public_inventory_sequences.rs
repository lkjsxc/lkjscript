//! Generalized sequence semantics require exact copied-product behavior witnesses.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_data_sequences::";
pub(super) const REQUIRED: [&str; 5] = [
    "native_data_sequences_generic_export_finite_model_and_detached_execution",
    "native_data_sequences_immutable_reads_survive_replacement_removal_disposal_and_transfer",
    "native_data_sequences_owned_replacement_preserves_displaced_custody",
    "negative::native_data_sequences_reject_hidden_types_active_loans_and_unsafe_substitutions",
    "native_data_sequences_bounds_and_fuel_refusals_join_cleanup_and_recover",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required native data sequence behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
