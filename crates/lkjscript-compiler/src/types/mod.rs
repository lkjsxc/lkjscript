//! Reusable type representation and annotation parsing.

mod enum_ids;
mod prelude;
mod ty;

pub use enum_ids::{EnumId, RuntimeLayoutId, VariantFieldId, VariantId};
pub(crate) use prelude::*;
pub(crate) use ty::parse_one;
pub use ty::Type;
