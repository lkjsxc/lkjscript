mod constant;
mod error;
mod ids;
mod instruction;
mod memory;
mod metadata;
mod program;
mod runtime;
mod terminator;
mod types;

pub use constant::Constant;
pub use error::{IrError, Result};
pub use ids::{
    BindingId, BlockId, EnumId, FunctionId, ImplId, LoanId, PlaceId, ProductId, RuntimeLayoutId,
    TraitId, ValueId, VariantFieldId, VariantId,
};
pub use instruction::{
    BlockParameter, BorrowKind, CallTarget, FailureBehavior, FrameLocal, FrameState, Instruction,
    InstructionKind, InstructionMetadata, Safepoint,
};
pub use memory::{
    MemoryAliasing, MemoryContention, MemoryDestruction, MemoryIdentity, MemoryLocality,
    MemoryMode, MemoryMultiplicity, MemoryObligationSubject, MemoryPortability, MemoryStorage,
    SsaMemoryInventory, SsaMemoryObligation,
};
pub use metadata::{
    EffectSet, EnumFieldMetadata, EnumLayoutFacts, EnumMetadata, EnumVariantMetadata, ProductField,
    ProductMetadata, SourceMetadata,
};
pub use program::{
    Block, BytecodeBlockLink, BytecodeInstructionLink, BytecodeLinkMetadata, Function,
    FunctionBytecodeLink, PlaceMetadata, Program,
};
pub use runtime::RuntimeOp;
pub use terminator::{BlockMetadata, StructuredOutcome, Terminator};
pub use types::{
    GenericInstantiation, ImplMetadata, Origin, Signature, SsaType, TraitBound, TraitMetadata,
    TraitRole, TraitWitness, TraitWitnessKind, TypeSubstitution,
};
