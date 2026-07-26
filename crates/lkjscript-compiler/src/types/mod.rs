//! Reusable type representation and annotation parsing.

mod enum_ids;
mod ty;

pub use enum_ids::{EnumId, VariantFieldId, VariantId};
pub(crate) use ty::parse_one;
pub use ty::Type;
