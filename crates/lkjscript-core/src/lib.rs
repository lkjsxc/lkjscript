//! Pure language core: values, validated bytecode, limits, and outcomes.

mod chunk;
mod error;
mod limits;
mod opcode;
mod outcome;
mod validation;
mod value;

pub use chunk::{
    Chunk, ConstId, Constant, FunctionProto, ProductFieldRef, ProductId, ProductMetadata,
};
pub use error::{Error, ErrorClass, Result};
pub use limits::{
    ExecutionConfig, Limits, ValidationLimits, MAX_BYTECODE_METADATA_BYTES,
    MAX_BYTECODE_TABLE_ENTRIES, MAX_CHILDREN, MAX_CHUNK_ENCODED_BYTES, MAX_CONSTANT_DATA_BYTES,
    MAX_DIR_CHILDREN, MAX_FUNCTION_CODE_BYTES, MAX_LIST_EQUAL_STEPS, MAX_NEST_DEPTH,
    MAX_PRODUCT_FIELDS, MAX_TOKENS_PER_FILE, MAX_TOPLEVEL_FORMS,
};
pub use opcode::{ControlFlow, DecodedInstruction, Op, OpInfo, StackEffect};
pub use outcome::{ExecutionOutcome, HostError, OwnedValue, ResourceLimitKind, Trap};
pub use validation::{validate_chunk, ValidatedChunk};
pub use value::{HeapObj, Value, MAX_SMALL_I64, MIN_SMALL_I64};
