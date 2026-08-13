//! Type expressions: exact I64/F64 numerics, parametric types, no Any.

use std::collections::HashMap;

use lkjscript_core::{CapabilityKind, ProductId, ResourceKind};

use crate::types::EnumId;

mod display;
mod model;
mod parse;

pub(crate) use model::*;
pub(crate) use parse::parse_one;

#[cfg(test)]
mod tests;
