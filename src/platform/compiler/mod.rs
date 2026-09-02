//! Declaration-sized compiler units for normalized Graph 7 authority.
//!
//! This boundary is intentionally separate from the predecessor recursive-AST compiler. It reads
//! exact stable-ID records through the same revision-pinned surfaces as validation and emits
//! typed dense operands. The public artifact and runtime still use the predecessor compiler until
//! the dependency-closed direct cutover.

#![allow(
    unused_imports,
    reason = "private compiler exports become artifact and runtime consumers at the Graph 7 cutover"
)]

mod artifact;
mod cache;
mod link;
mod lower;
pub(crate) mod manifest;
pub(crate) mod unit;

pub(crate) use artifact::{
    ARTIFACT_BUNDLE_CHECKSUM_DOMAIN, ARTIFACT_BUNDLE_DIGEST_DOMAIN, ARTIFACT_BUNDLE_END_MAGIC,
    ARTIFACT_BUNDLE_MAGIC, ARTIFACT_CLOSURE_DIGEST_DOMAIN, ARTIFACT_MANIFEST_ENVELOPE_DOMAIN,
    ARTIFACT_MANIFEST_MAGIC, MAXIMUM_ARTIFACT_BUNDLE_BYTES,
};
pub use artifact::{
    ARTIFACT_BUNDLE_CONTRACT_IDENTITY, ARTIFACT_CONTRACT_VERSION,
    ARTIFACT_MANIFEST_CONTRACT_IDENTITY, ArtifactBundleDigest, ArtifactClosureDigest,
    ArtifactLoadWork, ArtifactManifest, ArtifactManifestDigest, ArtifactPackage,
    ArtifactRuntimeOwner, EncodedArtifact, LoadedArtifact, load_artifact,
};
pub use cache::{
    CachedCompilation, CompilationBuildProfile, CompilationBuildReceipt, CompilationBuildWork,
    CompilationValidationReceipt, build_clean, build_incremental, load_current_compilation,
    validate_current_compilation,
};
pub use link::{ArtifactLinkReceipt, ArtifactLinkWork, link_artifact};
pub use lower::{CompilationReceipt, CompilationWork, compile_unit};
pub use manifest::{
    COMPILATION_MANIFEST_CONTRACT_IDENTITY, COMPILATION_MANIFEST_CONTRACT_VERSION,
    CompilationBinding, CompilationManifest, CompilationManifestDigest, CompilerUnitObjectDigest,
};
pub(crate) use manifest::{COMPILATION_MANIFEST_ENVELOPE_DOMAIN, COMPILATION_MANIFEST_MAGIC};
pub use unit::{
    BYTECODE_CONTRACT_IDENTITY, BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_IDENTITY,
    COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload, CompilationSource, CompilationUnit,
    CompilationUnitKey, CompiledCode, CompiledInstruction, OptimizationPolicy,
};
pub(crate) use unit::{
    COMPILER_UNIT_ENVELOPE_DOMAIN, COMPILER_UNIT_KEY_DOMAIN, COMPILER_UNIT_MAGIC,
};

#[cfg(test)]
pub(crate) mod tests;
