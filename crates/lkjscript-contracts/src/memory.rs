mod catalog_artifacts;
mod catalog_runtime;
mod catalog_structural;
mod catalog_targets;
mod catalog_values;
mod model;
mod templates;
mod templates_runtime;
mod templates_scalar;
mod templates_special;
mod witness;

pub use model::MemoryObligation;
pub use witness::{
    ExecutableMemoryWitnessFacts, MemoryWitnessCodec, MemoryWitnessContention, MemoryWitnessCopy,
    MemoryWitnessDomain, MemoryWitnessDrop, MemoryWitnessEquality, MemoryWitnessListElement,
    MemoryWitnessMode, MemoryWitnessOperation, MemoryWitnessPortability, MemoryWitnessRoot,
    MemoryWitnessSize,
};

use templates_special::resource;

/// Returns deterministic Current evidence and accepted candidate plans.
/// The registered contract identifies its schema; semantic authorities remain separate.
pub fn memory_obligations() -> Vec<MemoryObligation> {
    let mut records = Vec::new();
    records.extend_from_slice(catalog_values::VALUES);
    records.extend_from_slice(catalog_structural::STRUCTURAL);
    records.extend_from_slice(catalog_artifacts::ARTIFACTS);
    records.extend_from_slice(catalog_runtime::RUNTIME);
    records.extend_from_slice(catalog_targets::TARGETS);
    records.extend(ResourceKind::ALL.into_iter().map(|kind| {
        resource(
            kind.as_str(),
            "typed host acquisition",
            "resource lifecycle suites",
        )
    }));
    records.sort_unstable_by_key(|record| record.identity);
    debug_assert!(records
        .windows(2)
        .all(|pair| pair[0].identity != pair[1].identity));
    records
}

use crate::ResourceKind;

#[cfg(test)]
mod tests;
