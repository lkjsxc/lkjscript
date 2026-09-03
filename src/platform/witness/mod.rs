//! Private owner-granular Graph 8 validation witness.
//!
//! The witness is wholly derived from canonical kernel records. A full rebuild is retained as the
//! independent authority for later delta-update and impact-planning work.

#![allow(
    unused_imports,
    reason = "private witness exports become crate consumers at the Graph 8 repository cutover"
)]

mod codec;
pub(crate) mod contract;
mod digest;
mod entry;
mod full;
mod ownership;
mod summary;
mod summary_build;

pub(crate) use codec::bind_witness_manifest;
pub use codec::{
    decode_owner_summary, decode_witness_manifest, encode_owner_summary, encode_witness_manifest,
};
pub(crate) use contract::{MAXIMUM_RELATION_PREFIX_ITEMS, MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS};
pub use digest::{
    OwnerSummaryDigest, SemanticDigest, ValidationCertificateDigest, ValidationWitnessDigest,
    ValidatorContractDigest,
};
pub use entry::*;
pub use full::{FullWitness, WitnessBuildReport, WitnessEntries, rebuild_full_witness};
pub(crate) use ownership::ownership_contributions;
pub use summary::{OwnerSummary, SummaryBinding, ValidationWitnessManifest, WitnessRoots};
pub(crate) use summary_build::{
    SummaryRead, aggregation_children, rebuild_selected_owner_summaries,
};

fn witness_error(
    class: crate::platform::diagnostic::DiagnosticClass,
    code: &str,
    message: impl Into<String>,
) -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests;
