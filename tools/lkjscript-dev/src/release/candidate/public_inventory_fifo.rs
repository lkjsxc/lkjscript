//! Require the complete maintained FIFO behavior family before final-byte acceptance.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_owned_fifo::";
pub(super) const REQUIRED: [&str; 5] = [
    "native_owned_fifo_exact_generic_library_exhaustive_interleavings_and_source_free_results",
    "native_owned_fifo_traps_preserve_complete_input_admission_cleanup_and_following_invocations",
    "native_owned_fifo_finite_fuel_refusal_joins_detached_custody_without_partial_results",
    "negative::native_owned_fifo_rejects_read_escape_and_protected_consumption_without_publication",
    "oracle::native_owned_fifo_oracle_distinguishes_lifo_and_nonconsuming_peek",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required native FIFO behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
