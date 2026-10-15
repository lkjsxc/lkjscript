//! Source-tied read views require every actual public witness and complete namespace outcomes.
use super::{DevError, require};
use std::collections::BTreeSet;

pub(super) const PREFIX: &str = "native_declarations::native_associated_read_views::";
pub(super) const REQUIRED: [&str; 4] = [
    "native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes",
    "native_associated_read_views_source_alias_path_custody_and_complete_category_refusals",
    "native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody",
    "native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required associated read-view witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}
