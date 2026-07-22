//! Reusable type representation and annotation parsing.

mod ty;

pub(crate) use ty::parse_one;
pub use ty::Type;
