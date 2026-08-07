//! Pure language core: values, validated bytecode, execution policy, and outcomes.

mod bytecode;
mod error;
mod limits;
mod numeric_conversion;
mod opcode;
mod outcome;
mod prelude;
mod resource_table;
mod structural;
mod validation;
mod value;

pub use bytecode::{
    runtime_product_contract_identity, CallWitnessSite, Chunk, ConstId, Constant,
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, EnumVariantRef, FailureCleanupAction, FailureCleanupId,
    FailureCleanupInterner, FailureCleanupNode, FailureCleanupRange, FailureCleanupRoots,
    FunctionProto, GlobalId, InstalledMemoryWitness, InstalledMemoryWitnessGroup,
    InstalledMemoryWitnessGroupMember, MemoryPlanId, MemoryWitnessBinding, MemoryWitnessGroupId,
    MemoryWitnessId, MemoryWitnessParameter, MemoryWitnessValueKind, ProductFieldRef, ProductId,
    ProductMetadata, RegionProductFieldKind, ResourceReturnKind, RuntimeLayoutId,
    StructuralAggregateFieldRef, StructuralDestinationFieldRef, StructuralDestinationId,
    StructuralDestinationMetadata, StructuralFieldMetadata, StructuralFieldRoute,
    StructuralLayoutId, StructuralLayoutKind, StructuralLayoutMetadata, StructuralPayloadRef,
    StructuralRepresentationId, StructuralRepresentationMetadata, StructuralSliceExt,
    StructuralStorage, StructuralTypeId, StructuralTypeKind, StructuralTypeMetadata,
    StructuralTypeMode, StructuralValueCategory, StructuralVariantLayout, UniqueValueKind,
    VariantFieldId, VariantId,
};
pub use error::{Error, ErrorClass, Result};
pub use limits::ExecutionConfig;
pub use lkjscript_contracts::{sha256, MemoryWitnessOperation, Sha256};
pub use numeric_conversion::{
    f64_from_i64_exact, f64_from_i64_rounded, i64_from_f64_exact, i64_from_f64_trunc, NumericError,
};
pub use opcode::{
    ControlFlow, DecodedInstruction, DecodedOperand, Op, OpInfo, OperandLayout, StackEffect,
};
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
pub use resource_table::{
    BorrowedReservation, EmergencyObligations, OwnedReservation, ProviderId,
    ResourceCleanupAttempt, ResourceCleanupReport, ResourceKey, ResourceObservation,
    ResourceOwnership, ResourceState, ResourceTable, ResourceTableConfigError, ResourceTableError,
    ResourceTableLimit, ResourceTableLimits, ResourceTableStats, ResourceTokenParts, ScopeId,
};
pub use structural::{
    product_layout_identity, product_semantic_identity, ByteVectorKey, BytesKey, CheckedU64Range,
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
    StructuralBorrowKey, StructuralDestinationKey, StructuralDisposeReport, StructuralError,
    StructuralEvent, StructuralEventKind, StructuralEventLog, StructuralFieldPath, StructuralImage,
    StructuralImageConversionFailure, StructuralInitializationFailure, StructuralKind,
    StructuralLimit, StructuralLimits, StructuralNode, StructuralNodePayload, StructuralNodeRecord,
    StructuralNodeView, StructuralOwnerKind, StructuralProjection, StructuralPublishFailure,
    StructuralRootOwnership, StructuralRootState, StructuralRootTable, StructuralRootTableError,
    StructuralRootTableLimit, StructuralRootTableLimits, StructuralRootTableStats,
    StructuralRuntime, StructuralRuntimeId, StructuralRuntimeMetrics, StructuralSealResult,
    StructuralType, StructuralValueError, StructuralValueKey, StructuralValueLimit,
    StructuralValueRuntime, StructuralValueRuntimeLimits, StructuralValueRuntimeMetrics,
    StructuralViewKey, TypedPool, UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreError,
    UniqueStoreId, UniqueStoreLeak, UniqueStoreLimits, UniqueStoreStats, WeakSealedRef,
    DEFAULT_STRUCTURAL_TREE_NODES,
};
pub use validation::{
    bind_prepared_identity, validate_chunk, validated_bytecode_identity, ValidatedBytecodeIdentity,
    ValidatedChunk, ValidationPolicy,
};
pub use value::{CapabilityKind, ResourceKind, Value};
