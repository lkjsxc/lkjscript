//! Mixed storage contracts require complete copied-product composition and refusal evidence.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_indexed_data::";
pub(super) const REQUIRED: [&str; 3] = [
    "native_indexed_data_separate_packages_snapshots_forwarded_witnesses_and_detached_execution",
    "native_indexed_data_rejects_wrong_categories_modes_scopes_and_phantom_actuals",
    "native_indexed_data_traps_refusals_cleanup_and_recovery",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required native indexed storage behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}
