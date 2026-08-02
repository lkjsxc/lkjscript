//! Pure language core: values, validated bytecode, limits, and outcomes.

mod budget;
mod bytecode;
mod error;
mod limits;
mod numeric_conversion;
mod opcode;
mod outcome;
mod prelude;
mod profile;
mod resource_table;
mod sha256;
mod structural;
mod validation;
mod value;

pub use budget::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetLedger, BudgetPath,
    BudgetPrefix, BudgetRejectedEvent, BudgetScope, Reservation, ReservationId,
    ReservationJournalRecord, ReservationState, ResourceCategory, ResourceDiagnostic,
    ResourceUsage, MAX_BUDGET_JOURNAL_ENTRIES, MAX_BUDGET_PATH_DEPTH,
};
pub use bytecode::{
    runtime_product_contract_identity, CallWitnessSite, Chunk, ConstId, Constant,
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, EnumVariantRef, FailureCleanupAction, FailureCleanupPlan,
    FailureCleanupRange, FunctionProto, InstalledMemoryWitness, MemoryPlanId, MemoryWitnessBinding,
    MemoryWitnessId, MemoryWitnessParameter, MemoryWitnessValueKind, ProductFieldRef, ProductId,
    ProductMetadata, RegionProductFieldKind, ResourceReturnKind, RuntimeLayoutId,
    StructuralAggregateFieldRef, StructuralDestinationFieldRef, StructuralDestinationId,
    StructuralDestinationMetadata, StructuralFieldMetadata, StructuralFieldRoute,
    StructuralLayoutId, StructuralLayoutKind, StructuralLayoutMetadata, StructuralPayloadRef,
    StructuralRepresentationId, StructuralRepresentationMetadata, StructuralStorage,
    StructuralTypeId, StructuralTypeKind, StructuralTypeMetadata, StructuralTypeMode,
    StructuralValueCategory, StructuralVariantLayout, UniqueValueKind, VariantFieldId, VariantId,
    MAX_CALL_WITNESS_SITES, MAX_MEMORY_WITNESSES, MAX_MEMORY_WITNESS_DEPENDENCIES,
    MAX_MEMORY_WITNESS_PARAMETERS, MAX_STRUCTURAL_DESTINATIONS, MAX_STRUCTURAL_LAYOUTS,
    MAX_STRUCTURAL_LAYOUT_FIELDS, MAX_STRUCTURAL_OPERATION_REFS, MAX_STRUCTURAL_REPRESENTATIONS,
    MAX_STRUCTURAL_TYPES,
};
pub use error::{Error, ErrorClass, Result};
pub use limits::{
    ExecutionConfig, Limits, ValidationLimits, MAX_BULK_IO_BYTES, MAX_BYTECODE_METADATA_BYTES,
    MAX_BYTECODE_TABLE_ENTRIES, MAX_BYTE_STORAGE_BYTES, MAX_CHILDREN, MAX_CHUNK_ENCODED_BYTES,
    MAX_CONSTANT_DATA_BYTES, MAX_DIR_CHILDREN, MAX_FUNCTION_CODE_BYTES, MAX_LIST_EQUAL_STEPS,
    MAX_NEST_DEPTH, MAX_PRODUCT_FIELDS, MAX_TOKENS_PER_FILE, MAX_TOPLEVEL_FORMS,
};
pub use numeric_conversion::{
    f64_from_i64_exact, f64_from_i64_rounded, i64_from_f64_exact, i64_from_f64_trunc, NumericError,
};
pub use opcode::{ControlFlow, DecodedInstruction, Op, OpInfo, StackEffect};
pub use outcome::{
    decode_execution_outcome, encode_execution_outcome, CleanupFailure, CleanupFailureLimits,
    CleanupFailures, CleanupPhase, CleanupSubject, ExecutionOutcome, ExecutionOutcomeCodecLimits,
    HostError, OwnedValue, ResourceLimitKind, SealedSemanticDagBorrow,
    SealedSemanticDagBorrowFailure, SealedSemanticDagError, SealedSemanticDagFailure,
    SealedSemanticDagMetrics, SealedSemanticDagOwner, SealedSemanticDagReleaseFailure,
    SealedSemanticDagReleaseReport, SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode,
    SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot, SemanticDagType,
    StructuralSnapshotLimits, StructuralSnapshotMetrics, Trap, DEFAULT_MAX_CLEANUP_FAILURES,
    DEFAULT_MAX_CLEANUP_FAILURE_BYTES, MAX_CLEANUP_FAILURES, MAX_CLEANUP_FAILURE_BYTES,
    MAX_STRUCTURAL_SNAPSHOT_BYTES, MAX_STRUCTURAL_SNAPSHOT_DEPTH, MAX_STRUCTURAL_SNAPSHOT_FIELDS,
    MAX_STRUCTURAL_SNAPSHOT_NODES, MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES,
    MAX_STRUCTURAL_SNAPSHOT_WORK,
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
pub use structural::{
    product_layout_identity, product_semantic_identity, ByteVectorKey, BytesKey, CheckedU32Range,
    DestinationCleanupReport, DomainClass, DomainKey, InlineStructuralValue, InvalidUniqueKeyWord,
    InvalidUniqueStoreLimits, LayoutIdentity, LocalNodeId, PathKey, PoolId, PoolMetrics,
    PoolPartition, RegionMetrics, RegionOwner, RegionProductArena, RegionProductArenaId,
    RegionProductError, RegionProductKey, RegionProductLimits, RegionProductMetrics, RegionRef,
    RegionReleaseReport, RegionStore, RootClass, RootKey, SealFailure, SealedBorrow, SealedBuilder,
    SealedOwner, SealedRef, SealedRegionMetrics, SealedRegionStore, SealedReleaseReport,
    SealedUpgrade, SegmentedListArena, SegmentedListArenaId, SegmentedListArenaLimits,
    SegmentedListError, SegmentedListKey, SegmentedListLimit, SegmentedListMetrics,
    SemanticChildren, SemanticPayload, SemanticTypeIdentity, SemanticValue, StaticArtifactPayload,
    StaticBytes, StaticStructuralArtifact, StaticStructuralLeaf, StructuralBorrow,
    StructuralBorrowKey, StructuralDestinationKey, StructuralError, StructuralEvent,
    StructuralEventKind, StructuralEventLog, StructuralFieldPath, StructuralImage,
    StructuralImageConversionFailure, StructuralInitializationFailure, StructuralKind,
    StructuralLimit, StructuralLimits, StructuralNode, StructuralNodePayload, StructuralNodeRecord,
    StructuralNodeView, StructuralProjection, StructuralPublishFailure, StructuralRootOwnership,
    StructuralRootState, StructuralRootTable, StructuralRootTableError, StructuralRootTableLimit,
    StructuralRootTableLimits, StructuralRootTableStats, StructuralRuntime, StructuralRuntimeId,
    StructuralRuntimeMetrics, StructuralType, StructuralValueError, StructuralValueKey,
    StructuralValueLimit, StructuralValueRuntime, StructuralValueRuntimeLimits,
    StructuralValueRuntimeMetrics, StructuralViewKey, TypedPool, UniqueKeyWord, UniqueLayout,
    UniqueStore, UniqueStoreError, UniqueStoreId, UniqueStoreLeak, UniqueStoreLimits,
    UniqueStoreStats, WeakSealedRef,
};
pub use validation::{validate_chunk, ValidatedChunk};
pub use value::{CapabilityKind, ResourceKind, Value};
