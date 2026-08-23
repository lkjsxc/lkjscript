//! Private owner-granular Graph 5 validation witness.
//!
//! The witness is wholly derived from canonical kernel records. A full rebuild is retained as the
//! independent authority for later delta-update and impact-planning work.

#![allow(
    unused_imports,
    reason = "private witness exports become crate consumers at the Graph 5 repository cutover"
)]

mod codec;
pub(crate) mod contract;
mod digest;
mod entry;
mod full;
mod ownership;
mod summary;
mod summary_build;

pub use codec::{
    decode_owner_summary, decode_witness_manifest, encode_owner_summary, encode_witness_manifest,
};
pub use digest::{
    OwnerSummaryDigest, SemanticDigest, ValidationCertificateDigest, ValidationWitnessDigest,
    ValidatorContractDigest,
};
pub use entry::*;
pub use full::{FullWitness, WitnessBuildReport, WitnessEntries, rebuild_full_witness};
pub(crate) use ownership::ownership_contributions;
pub use summary::{OwnerSummary, SummaryBinding, ValidationWitnessManifest, WitnessRoots};

fn witness_error(
    class: crate::platform::diagnostic::DiagnosticClass,
    code: &str,
    message: impl Into<String>,
) -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests;
