mod constant;
mod error;
mod ids;
mod instruction;
mod instruction_metadata;
mod instruction_operands;
mod memory;
mod metadata;
mod program;
mod runtime;
mod structural;
mod terminator;
mod types;

pub use constant::Constant;
pub use error::{IrError, Result};
pub use ids::{
    BindingId, BlockId, EnumId, FailureCleanupId, FunctionId, ImplId, LoanId, MemoryPlanId,
    MemoryWitnessGroupId, MemoryWitnessId, PlaceId, ProductId, RuntimeLayoutId, StructuralLayoutId,
    StructuralRepresentationId, StructuralTypeId, TraitId, ValueId, VariantFieldId, VariantId,
};
pub use instruction::{BlockParameter, BorrowKind, CallTarget, Instruction, InstructionKind};
pub use instruction_metadata::{
    FailureBehavior, FailureCleanupAction, FailureCleanupInterner, FailureCleanupNode,
    FailureCleanupRoots, FrameLocal, FrameState, InstructionMetadata,
};
pub use memory::{
    DropEventKind, DropGlueIdentity, MemoryAliasing, MemoryContention, MemoryDestruction,
    MemoryIdentity, MemoryLocality, MemoryMode, MemoryMultiplicity, MemoryObligationSubject,
    MemoryPortability, MemoryStorage, SsaMemoryInventory, SsaMemoryObligation,
};
pub use metadata::{
    EffectSet, EnumFieldMetadata, EnumLayoutFacts, EnumMetadata, EnumVariantMetadata, ProductField,
    ProductMetadata, SourceMetadata,
};
pub use program::{
    Block, BytecodeBlockLink, BytecodeInstructionLink, BytecodeLinkMetadata, FailureCleanupActions,
    Function, FunctionBytecodeLink, PlaceMetadata, Program,
};
pub use runtime::RuntimeOp;
pub use structural::{
    runtime_product_contract_identity, runtime_product_identity, runtime_product_layout_identity,
    runtime_product_semantic_type, runtime_structural_semantic_type, runtime_structural_type,
    MemoryWitnessDescriptor, MemoryWitnessGroupDescriptor, MemoryWitnessGroupMember,
    RegionProductMetadata, StructuralDropGlueIdentity, StructuralLayoutKind,
    StructuralLayoutMetadata, StructuralMemoryMetadata, StructuralRepresentationMetadata,
    StructuralStorage, StructuralTypeMetadata, StructuralTypeMode, StructuralValueCategory,
    StructuralVariantLayout, MAX_MEMORY_WITNESSES, MAX_MEMORY_WITNESS_DEPENDENCIES,
    MAX_MEMORY_WITNESS_GROUPS, MAX_MEMORY_WITNESS_PARAMETERS, MAX_REGION_PRODUCTS,
    MAX_STRUCTURAL_LAYOUTS, MAX_STRUCTURAL_LAYOUT_FIELDS, MAX_STRUCTURAL_REPRESENTATIONS,
    MAX_STRUCTURAL_TYPES,
};
pub use terminator::{BlockMetadata, StructuredOutcome, Terminator};
pub use types::{
    GenericInstantiation, ImplMetadata, MemoryWitnessBinding, MemoryWitnessParameter, Origin,
    Signature, SsaType, TraitBound, TraitMetadata, TraitRole, TraitWitness, TraitWitnessKind,
    TypeSubstitution,
};
