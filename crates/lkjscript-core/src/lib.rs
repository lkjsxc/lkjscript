//! Pure language core: values, validated bytecode, limits, and outcomes.

mod budget;
mod chunk;
mod error;
mod gc;
mod limits;
mod opcode;
mod outcome;
mod profile;
mod sha256;
mod validation;
mod value;

pub use budget::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetLedger, BudgetPath,
    BudgetScope, Reservation, ReservationId, ReservationState, ResourceCategory,
    ResourceDiagnostic, ResourceUsage, MAX_BUDGET_PATH_DEPTH,
};
pub use chunk::{
    Chunk, ConstId, Constant, FunctionProto, ProductFieldRef, ProductId, ProductMetadata,
};
pub use error::{Error, ErrorClass, Result};
pub use gc::{GcConfig, GcHeap, GcLimit, GcStats};
pub use limits::{
    ExecutionConfig, Limits, ValidationLimits, MAX_BUFFER_BYTES, MAX_BULK_IO_BYTES,
    MAX_BYTECODE_METADATA_BYTES, MAX_BYTECODE_TABLE_ENTRIES, MAX_CHILDREN, MAX_CHUNK_ENCODED_BYTES,
    MAX_CONSTANT_DATA_BYTES, MAX_DIR_CHILDREN, MAX_FUNCTION_CODE_BYTES, MAX_LIST_EQUAL_STEPS,
    MAX_NEST_DEPTH, MAX_PRODUCT_FIELDS, MAX_TOKENS_PER_FILE, MAX_TOPLEVEL_FORMS,
};
pub use opcode::{ControlFlow, DecodedInstruction, Op, OpInfo, StackEffect};
pub use outcome::{ExecutionOutcome, HostError, OwnedValue, ResourceLimitKind, Trap};
pub use profile::{
    InvalidCeiling, ResourceCeilings, ResourceProfile, ResourceProfileIdentity,
    ResourceProfileName, UnknownResourceProfile, IMPLEMENTATION_MAXIMA_VERSION,
    RESOURCE_PROFILE_SCHEMA, RESOURCE_PROFILE_VERSION,
};
pub use sha256::sha256;
pub use validation::{validate_chunk, ValidatedChunk};
pub use value::{HeapObj, Value, MAX_SMALL_I64, MIN_SMALL_I64};
