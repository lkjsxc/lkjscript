//! First compiler consumer. These are the production K-to-ELF modules.
//! This root has no source parser, elaborator, library loader or Rust host API.
pub mod check;
pub mod diagnostic;
pub(crate) mod facts;
pub mod k;
pub mod k_text;
pub mod kc;
pub mod lower;
pub mod native;
pub mod runtime;
