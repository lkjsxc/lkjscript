//! Incremental compilation is accepted only after actual copied-product edits and failures.
use super::{DevError, require};
use std::collections::BTreeSet;

const PREFIX: &str = "native_declarations::incremental_units::";
const REQUIRED: [&str; 3] = [
    "new_components_do_not_promote_ports_to_compiler_units",
    "new_command_targets_keep_exact_incremental_cache_and_detached_behavior",
    "new_port_type_failure_preserves_authority_and_cache_then_recovers",
];

pub(super) fn admit(all: &BTreeSet<String>) -> Result<(), DevError> {
    for suffix in REQUIRED {
        require(
            all.contains(&format!("{PREFIX}{suffix}")),
            &format!("required incremental compiler behavior witness is absent: {suffix}"),
        )?;
    }
    Ok(())
}

pub(super) fn selected(name: &str) -> bool {
    name.starts_with(PREFIX)
}
