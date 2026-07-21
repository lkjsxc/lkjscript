//! Mandatory type language: Type AST, prelude, and checking.

mod check;
mod infer;
mod prelude;
mod prelude_sys;
mod ty;

pub use check::typecheck_program;
