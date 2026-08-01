//! Validated authority for the one canonical source language.
//!
//! Parsing, loading, identity, and construction remain private. The only
//! public source representation is the opaque [`ValidatedSourceTree`].

mod api;
mod diagnostics;
mod format;
mod identity;
mod load;
mod model;
pub(crate) mod module_names;
mod modules;
pub(crate) use modules::{
    public_memory_witness_requirements as module_memory_witness_requirements,
    public_names as module_public_names, PublicMemoryWitnessRequirement,
};
mod parse;
mod validate;

#[cfg(test)]
pub(crate) use api::validate_source_set_for_analysis;
pub(crate) use api::{
    ensure_source_path_for_compiler, load_for_compiler_with_budget, load_for_protocol,
    load_with_metrics_and_budget, rebuild_staged_sources, validate_for_compiler_with_budget,
    ValidatedSourceParts,
};
pub use api::{load, validate, ValidatedSourceTree};
pub use diagnostics::{
    DiagnosticCategory, DiagnosticCertainty, DiagnosticSeverity, RelatedSourceSpan,
    SourceDiagnostic, SourceOrigin, SourcePosition, SourceResult, SourceSpan,
};
pub(crate) use format::{format_f64, format_file, format_node_identity, format_node_source};
pub(crate) use identity::enum_member_identity;
pub use identity::{
    DeclarationKey, DeclarationKind, DeclarationSummary, NodeId, NodeKind, NodeSummary, RevisionId,
    SourceIdentity, SourceTreeIdentity, StaleNodeId,
};
pub use load::validate_source_directory_tree;
pub(crate) use model::{Expr, SourceFile, SourceNode, SyntaxKind, Token, TokenKind};
pub(crate) use parse::is_source_identifier;
pub(crate) use validate::SourceFoundationBudget;

/// Always-enforced maximum for exact bytes in one source file.
pub const FOUNDATION_MAX_SOURCE_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Always-enforced maximum for exact bytes in a source closure.
pub const FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
/// Always-enforced maximum for source units in a source closure.
pub const FOUNDATION_MAX_SOURCE_UNITS: u64 = 65_536;
/// Always-enforced maximum for entries scanned by a source-tree check.
pub const FOUNDATION_MAX_SOURCE_TREE_ENTRIES: u64 = 65_536;

#[cfg(test)]
mod tests;
