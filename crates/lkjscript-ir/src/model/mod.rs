mod constant;
mod error;
mod ids;
mod instruction;
mod metadata;
mod program;
mod runtime;
mod terminator;
mod types;

pub use constant::Constant;
pub use error::{IrError, Result};
pub use ids::{
    BindingId, BlockId, FunctionId, ImplId, LoanId, PlaceId, ProductId, TraitId, ValueId,
};
pub use instruction::{
    BlockParameter, BorrowKind, CallTarget, FailureBehavior, FrameLocal, FrameState, Instruction,
    InstructionKind, InstructionMetadata, Safepoint,
};
pub use metadata::{EffectSet, ProductField, ProductMetadata, SourceMetadata};
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
