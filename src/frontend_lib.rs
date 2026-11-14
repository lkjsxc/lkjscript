//! Source-producer consumer. Every module is shared with the complete compiler.
//! Host filesystem access and native emission are outside this boundary.
pub mod ast;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod elaborate;
pub(crate) mod facts;
pub mod frontend;
pub mod generics;
pub mod k;
pub mod k_text;
pub mod names;
pub mod resolve;
pub mod source;
pub mod syntax;

pub use frontend::source_to_k;
