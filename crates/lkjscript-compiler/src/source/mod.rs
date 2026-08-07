//! Validated authority for the one canonical source language.
//!
//! Parsing, loading, identity, and construction remain private. The only
//! public source representation is the opaque [`ValidatedSourceTree`].

mod api;
mod constructor_identity;
mod diagnostics;
mod identity;
mod load;
mod model;
pub(crate) mod module_names;
mod modules;
pub(crate) use modules::public_names as module_public_names;
mod parse;
#[cfg(test)]
pub(crate) use parse::{
    invocation_count as parser_invocation_count,
    reset_invocation_count as reset_parser_invocation_count,
};
mod validate;

#[cfg(test)]
pub(crate) use api::validate_source_set_for_analysis;
pub(crate) use api::{
    ensure_source_path_for_compiler, load_with_metrics, validate_for_compiler, ValidatedSourceParts,
};
pub(crate) use api::{load, validate, ValidatedSourceTree};
pub(crate) use diagnostics::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourcePosition, SourceResult, SourceSpan,
};
pub(crate) use identity::{enum_member_identity, product_field_identity};
pub(crate) use identity::{DeclarationKey, DeclarationKind, DeclarationSummary};
pub(crate) use model::{Expr, SourceFile, SourceNode, SyntaxKind, Token, TokenKind};
pub(crate) use parse::is_source_identifier;

#[cfg(test)]
mod tests;
