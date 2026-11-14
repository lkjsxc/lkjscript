//! First compiler consumer. These are the production IR-to-ELF modules.
//! This root has no source parser, elaborator, library loader or Rust host API.
pub mod backend;
pub mod check;
pub mod diagnostic;
pub(crate) mod facts;
pub mod ir;
pub mod ir_text;
pub mod lower;
pub mod native;
pub mod result_wire;
pub mod runtime;
