//! Private owner-granular Graph 10 validation witness.
//!
//! The witness is wholly derived from canonical kernel records. A full rebuild is retained as the
//! independent authority for later delta-update and impact-planning work.

#![allow(
    unused_imports,
    reason = "private witness exports become crate consumers at the Graph 10 repository cutover"
)]

mod codec;
pub(crate) mod contract;
mod digest;
mod entry;
mod frozen_summary11;
mod full;
mod ownership;
mod summary;
mod summary_build;

pub(crate) use codec::bind_witness_manifest;
pub(crate) use codec::decode_historical_witness_manifest;
pub(crate) use codec::encode_witness_manifest_content;
pub(crate) use codec::{decode_historical_owner_summary, encode_owner_summary_content};
pub use codec::{
    decode_owner_summary, decode_witness_manifest, encode_owner_summary, encode_witness_manifest,
};
pub(crate) use contract::{MAXIMUM_RELATION_PREFIX_ITEMS, MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS};
pub use digest::{
    OwnerSummaryDigest, SemanticDigest, ValidationCertificateDigest, ValidationWitnessDigest,
    ValidatorContractDigest,
};
pub use entry::*;
#[cfg(test)]
pub(crate) use full::checked_full_validation_invocations;
pub(crate) use full::{CanonicalWitnessFacts, rebuild_canonical_facts};
pub(crate) use full::{
    CompletedSourceValidation, rebuild_full_witness_checked_for_source,
    rebuild_full_witness_from_source_validation,
};
pub use full::{FullWitness, WitnessBuildReport, WitnessEntries, rebuild_full_witness};
pub(crate) use full::{rebuild_full_witness_checked, rebuild_full_witness_with_limit};
pub(crate) use ownership::{ownership_contributions, ownership_contributions_with_bindings};
pub use summary::{OwnerSummary, SummaryBinding, ValidationWitnessManifest, WitnessRoots};
pub(crate) use summary_build::{
    SummaryRead, aggregation_children, aggregation_children_with_bindings,
    rebuild_selected_owner_summaries,
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

#[cfg(test)]
mod read_view_witness_tests;
