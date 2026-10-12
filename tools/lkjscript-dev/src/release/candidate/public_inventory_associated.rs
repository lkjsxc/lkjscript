//! Associated Owned members require the complete consumer-first copied-product witness.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_associated_blockread::";
pub(super) const REQUIRED: [&str; 1] =
    ["native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes"];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required associated Owned BlockRead witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
