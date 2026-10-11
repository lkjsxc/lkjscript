//! Opaque owned records require complete copied-product composition and failure witnesses.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_owned_nominals::";
pub(super) const REQUIRED: [&str; 5] = [
    "native_owned_nominals_export_generic_identity_and_detached_results",
    "native_owned_nominals_keep_representation_private",
    "native_owned_nominals_nested_borrows_preserve_source_until_consumption",
    "native_owned_nominals_admit_complete_layouts_and_phantom_arguments",
    "native_owned_nominals_transfer_traps_budget_join_cleanup_and_recovery",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required native owned nominal behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
