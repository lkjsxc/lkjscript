//! Type expressions: exact I64/F64 numerics, parametric types, no Any.

use std::collections::HashMap;

use lkjscript_core::CapabilityKind;

use crate::types::EnumId;

mod model;
mod parse;

pub use model::*;
pub use parse::parse_one;

#[cfg(test)]
mod tests;
