//! Type expressions: exact I64/F64 numerics, parametric types, no Any.

use std::collections::HashMap;

mod model;
mod parse;

pub use model::*;
pub use parse::parse_one;

#[cfg(test)]
mod tests;
