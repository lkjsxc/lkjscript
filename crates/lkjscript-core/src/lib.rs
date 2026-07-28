//! Pure language core: values, validated bytecode, limits, and outcomes.

mod budget;
mod bytecode;
mod error;
mod gc;
mod limits;
mod numeric_conversion;
mod opcode;
mod outcome;
mod prelude;
mod profile;
mod resource_table;
mod sha256;
mod unique;
mod validation;
mod value;

pub use budget::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetLedger, BudgetPath,
    BudgetPrefix, BudgetRejectedEvent, BudgetScope, Reservation, ReservationId,
    ReservationJournalRecord, ReservationState, ResourceCategory, ResourceDiagnostic,
    ResourceUsage, MAX_BUDGET_JOURNAL_ENTRIES, MAX_BUDGET_PATH_DEPTH,
};
pub use bytecode::{
    Chunk, ConstId, Constant, EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId,
    EnumMetadata, EnumVariantMetadata, EnumVariantRef, FailureCleanupAction, FailureCleanupPlan,
    FailureCleanupRange, FunctionProto, ProductFieldRef, ProductId, ProductMetadata,
    ResourceReturnKind, RuntimeLayoutId, UniqueValueKind, VariantFieldId, VariantId,
};
pub use error::{Error, ErrorClass, Result};
pub use gc::{GcConfig, GcHeap, GcLimit, GcStats};
pub use limits::{
    ExecutionConfig, Limits, ValidationLimits, MAX_BUFFER_BYTES, MAX_BULK_IO_BYTES,
    MAX_BYTECODE_METADATA_BYTES, MAX_BYTECODE_TABLE_ENTRIES, MAX_CHILDREN, MAX_CHUNK_ENCODED_BYTES,
    MAX_CONSTANT_DATA_BYTES, MAX_DIR_CHILDREN, MAX_FUNCTION_CODE_BYTES, MAX_LIST_EQUAL_STEPS,
    MAX_NEST_DEPTH, MAX_PRODUCT_FIELDS, MAX_TOKENS_PER_FILE, MAX_TOPLEVEL_FORMS,
};
pub use numeric_conversion::{
    f64_from_i64_exact, f64_from_i64_rounded, i64_from_f64_exact, i64_from_f64_trunc, NumericError,
};
pub use opcode::{ControlFlow, DecodedInstruction, Op, OpInfo, StackEffect};
pub use outcome::{
    CleanupFailure, CleanupFailureLimits, CleanupFailures, CleanupPhase, CleanupSubject,
    ExecutionOutcome, HostError, OwnedValue, ResourceLimitKind, Trap, DEFAULT_MAX_CLEANUP_FAILURES,
    DEFAULT_MAX_CLEANUP_FAILURE_BYTES, MAX_CLEANUP_FAILURES, MAX_CLEANUP_FAILURE_BYTES,
};
pub use prelude::*;
pub use profile::{
    InvalidCeiling, ResourceCeilings, ResourceProfile, ResourceProfileIdentity,
    ResourceProfileName, UnknownResourceProfile, RESOURCE_PROFILE_SCHEMA,
};
pub use resource_table::{
    BorrowedReservation, EmergencyObligations, OwnedReservation, ProviderId,
    ResourceCleanupAttempt, ResourceCleanupReport, ResourceKey, ResourceObservation,
    ResourceOwnership, ResourceState, ResourceTable, ResourceTableConfigError, ResourceTableError,
    ResourceTableLimit, ResourceTableLimits, ResourceTableStats, ResourceTokenParts, ScopeId,
};
pub use sha256::sha256;
pub use unique::{
    ByteVectorKey, BytesKey, InvalidUniqueKeyWord, InvalidUniqueStoreLimits, PathKey, StaticBytes,
    UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreError, UniqueStoreId, UniqueStoreLeak,
    UniqueStoreLimits, UniqueStoreStats,
};
pub use validation::{validate_chunk, ValidatedChunk};
pub use value::{CapabilityKind, HeapObj, ResourceKind, Value};
