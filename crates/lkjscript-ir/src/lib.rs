//! Backend-independent typed SSA, verification, evaluation, and baseline passes.
#![forbid(unsafe_code)]

mod eval;
mod memory;
mod model;
mod numeric_contract;
mod optimize;
pub mod prelude_contract;
mod specialize;
mod utf8_contract;
mod verify;

#[cfg(test)]
mod tests;

pub use eval::{
    evaluate, evaluate_observed, EvalConfig, EvalOutcome, EvalResourcePolicy,
    EvalStructuralObservation, EvalValue,
};
pub use memory::{derive_memory_inventory, verify_memory_inventory};
pub use model::{
    runtime_product_contract_identity, runtime_product_identity, runtime_product_layout_identity,
    runtime_product_semantic_type, runtime_structural_semantic_type, runtime_structural_type,
    BindingId, Block, BlockId, BlockMetadata, BlockParameter, BorrowKind, BytecodeBlockLink,
    BytecodeInstructionLink, BytecodeLinkMetadata, CallTarget, Constant, DropEventKind,
    DropGlueIdentity, EffectSet, EnumFieldMetadata, EnumId, EnumLayoutFacts, EnumMetadata,
    EnumVariantMetadata, FailureBehavior, FailureCleanupAction, FailureCleanupId,
    FailureCleanupPlan, FrameLocal, FrameState, Function, FunctionBytecodeLink, FunctionId,
    GenericInstantiation, ImplId, ImplMetadata, Instruction, InstructionKind, InstructionMetadata,
    IrError, LoanId, MemoryAliasing, MemoryContention, MemoryDestruction, MemoryIdentity,
    MemoryLocality, MemoryMode, MemoryMultiplicity, MemoryObligationSubject, MemoryPlanId,
    MemoryPortability, MemoryStorage, MemoryWitnessBinding, MemoryWitnessDescriptor,
    MemoryWitnessId, MemoryWitnessParameter, Origin, PlaceId, PlaceMetadata, ProductField,
    ProductId, ProductMetadata, Program, RegionProductMetadata, Result, RuntimeLayoutId, RuntimeOp,
    Signature, SourceMetadata, SsaMemoryInventory, SsaMemoryObligation, SsaType,
    StructuralDropGlueIdentity, StructuralLayoutId, StructuralLayoutKind, StructuralLayoutMetadata,
    StructuralMemoryMetadata, StructuralRepresentationId, StructuralRepresentationMetadata,
    StructuralStorage, StructuralTypeId, StructuralTypeMetadata, StructuralTypeMode,
    StructuralValueCategory, StructuralVariantLayout, StructuredOutcome, Terminator, TraitBound,
    TraitId, TraitMetadata, TraitRole, TraitWitness, TraitWitnessKind, TypeSubstitution, ValueId,
    VariantFieldId, VariantId, MAX_MEMORY_WITNESSES, MAX_MEMORY_WITNESS_DEPENDENCIES,
    MAX_MEMORY_WITNESS_PARAMETERS, MAX_REGION_PRODUCTS, MAX_STRUCTURAL_LAYOUTS,
    MAX_STRUCTURAL_LAYOUT_FIELDS, MAX_STRUCTURAL_REPRESENTATIONS, MAX_STRUCTURAL_TYPES,
};
pub use optimize::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, normalize_baseline, optimize, optimize_scheduled,
    optimize_scheduled_with_binder, simplify_branches, unreachable_blocks, verify_optimization,
    OptimizationCertificate, OptimizationCertificateRecord, OptimizationEditKind,
    OptimizationError, OptimizationFailureCode, OptimizationLimits, OptimizationStats,
    ScheduledOptimizationReport, VerifiedOptimizedProgram,
};
pub use specialize::{
    specialize_native_transport, NativeSpecializationStats, MAX_NATIVE_TRANSPORT_SPECIALIZATIONS,
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION,
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE,
};
pub use verify::{
    verify, VerifiedProgram, OWNERSHIP_VERIFY_MAX_WORK, SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION,
    SSA_VERIFY_MAX_CFG_WORK,
};
