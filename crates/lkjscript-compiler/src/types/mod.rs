//! Reusable type representation and annotation parsing.

mod enum_ids;
mod numeric_error;
mod ty;

pub use enum_ids::{EnumId, RuntimeLayoutId, VariantFieldId, VariantId};
pub(crate) use numeric_error::{numeric_error_layout, numeric_error_type, numeric_error_variant};
pub(crate) use ty::parse_one;
pub use ty::Type;
