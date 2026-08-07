//! Backend-independent typed SSA, verification, and normalization.
#![forbid(unsafe_code)]

#[cfg(any(test, feature = "test-oracle"))]
mod eval;
mod identity;
mod model;
mod normalize;
mod numeric_contract;
pub mod prelude_contract;
mod specialize;
#[cfg(any(test, feature = "test-oracle"))]
mod utf8_contract;
mod verify;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "test-oracle"))]
pub use eval::{
    evaluate, evaluate_observed, EvalConfig, EvalOutcome, EvalResourcePolicy,
    EvalStructuralObservation, EvalValue,
};
pub use identity::{verified_program_identity, VerifiedProgramIdentity};
pub use lkjscript_contracts::PreparedProgramIdentity;
pub use model::{
    runtime_product_contract_identity, runtime_product_identity, runtime_product_layout_identity,
    runtime_product_semantic_type, runtime_structural_semantic_type, runtime_structural_type,
    BindingId, Block, BlockId, BlockMetadata, BlockParameter, BorrowKind, BytecodeBlockLink,
    BytecodeInstructionLink, BytecodeLinkMetadata, CallTarget, Constant, DropEventKind,
    DropGlueIdentity, EffectSet, EnumFieldMetadata, EnumId, EnumLayoutFacts, EnumMetadata,
    EnumVariantMetadata, FailureBehavior, FailureCleanupAction, FailureCleanupActions,
    FailureCleanupId, FailureCleanupInterner, FailureCleanupNode, FailureCleanupRoots, FrameLocal,
    FrameState, Function, FunctionBytecodeLink, FunctionId, GenericInstantiation, ImplId,
    ImplMetadata, Instruction, InstructionKind, InstructionMetadata, IrError, LoanId, MemoryPlanId,
    MemoryWitnessBinding, MemoryWitnessDescriptor, MemoryWitnessGroupDescriptor,
    MemoryWitnessGroupId, MemoryWitnessGroupMember, MemoryWitnessId, MemoryWitnessParameter,
    Origin, PlaceId, PlaceMetadata, ProductField, ProductId, ProductMetadata, Program,
    RegionProductMetadata, Result, RuntimeLayoutId, RuntimeOp, Signature, SourceMetadata, SsaType,
    StructuralDropGlueIdentity, StructuralLayoutId, StructuralLayoutKind, StructuralLayoutMetadata,
    StructuralMemoryMetadata, StructuralRepresentationId, StructuralRepresentationMetadata,
    StructuralStorage, StructuralTypeId, StructuralTypeMetadata, StructuralTypeMode,
    StructuralValueCategory, StructuralVariantLayout, StructuredOutcome, Terminator, TraitBound,
    TraitId, TraitMetadata, TraitRole, TraitWitness, TraitWitnessKind, TypeSubstitution, ValueId,
    VariantFieldId, VariantId,
};
pub use normalize::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, normalize_baseline, simplify_branches,
    unreachable_blocks,
};
pub use specialize::{
    specialize_native_transport, NativeSpecializationStats, MAX_NATIVE_TRANSPORT_SPECIALIZATIONS,
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION,
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE,
};
pub use verify::{verify, VerifiedProgram};
