//! Single whole-chunk bytecode validation boundary.

mod control;
mod decode;
mod entry;
mod entry_capabilities;
mod enum_shape;
mod failure_cleanup;
mod instruction;
#[path = "model/merge.rs"]
mod merge;
mod prelude_shape;
mod shape;
mod shape_products;
mod shape_structural;

use crate::{
    Chunk, Constant, DecodedInstruction, EnumId, Error, FunctionProto, ProductId, Result, Value,
    VariantId,
};

include!("model/validated_chunk.rs");
include!("model/state.rs");

pub use entry::validate_chunk;

#[cfg(test)]
mod tests;
