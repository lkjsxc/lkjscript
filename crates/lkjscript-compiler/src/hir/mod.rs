//! Owned, resolved, and typed high-level intermediate representation.

use std::path::PathBuf;

pub use lkjscript_core::ProductId;

pub use crate::operation::Operation;
pub use crate::types::{EnumId, RuntimeLayoutId, Type, VariantFieldId, VariantId};

mod bindings;
mod effects;
mod enums;
mod expressions;
mod ids;
mod products;
mod traits;

pub use bindings::*;
pub use effects::*;
pub use enums::*;
pub use expressions::*;
pub use ids::*;
pub use products::*;
pub use traits::*;
