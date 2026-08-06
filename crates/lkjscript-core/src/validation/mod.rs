//! Single whole-chunk bytecode validation boundary.

mod control;
mod decode;
mod entry;
mod entry_capabilities;
mod enum_shape;
mod failure_cleanup;
#[path = "model/identity/mod.rs"]
mod identity;
mod instruction;
#[path = "model/merge.rs"]
mod merge;
mod policy;
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

pub use entry::{bind_prepared_identity, validate_chunk};
pub use identity::{validated_bytecode_identity, ValidatedBytecodeIdentity};
pub use policy::ValidationPolicy;

#[cfg(test)]
mod tests;
