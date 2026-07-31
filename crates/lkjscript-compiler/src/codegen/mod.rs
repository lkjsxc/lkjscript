//! Lower verified normalized typed SSA into validated-reference bytecode input.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{
    Chunk, Constant as BytecodeConstant, EnumConstructionRef,
    EnumFieldMetadata as BytecodeEnumFieldMetadata, EnumFieldRef, EnumId as BytecodeEnumId,
    EnumMetadata as BytecodeEnumMetadata, EnumVariantMetadata as BytecodeEnumVariantMetadata,
    EnumVariantRef, Error, FailureCleanupAction as BytecodeFailureCleanupAction,
    FailureCleanupPlan as BytecodeFailureCleanupPlan, FailureCleanupRange, FunctionProto,
    MemoryPlanId as BytecodeMemoryPlanId, Op, ProductFieldRef, ProductId as BytecodeProductId,
    ProductMetadata as BytecodeProductMetadata, RegionProductFieldKind, ResourceReturnKind, Result,
    RuntimeLayoutId as BytecodeLayoutId, StructuralAggregateFieldRef,
    StructuralDestinationFieldRef, StructuralDestinationId, StructuralDestinationMetadata,
    StructuralFieldMetadata, StructuralFieldRoute,
    StructuralLayoutId as BytecodeStructuralLayoutId,
    StructuralLayoutKind as BytecodeStructuralLayoutKind,
    StructuralLayoutMetadata as BytecodeStructuralLayoutMetadata, StructuralPayloadRef,
    StructuralRepresentationId as BytecodeStructuralRepresentationId,
    StructuralRepresentationMetadata as BytecodeStructuralRepresentationMetadata,
    StructuralStorage as BytecodeStructuralStorage, StructuralTypeId as BytecodeStructuralTypeId,
    StructuralTypeKind as BytecodeStructuralTypeKind,
    StructuralTypeMetadata as BytecodeStructuralTypeMetadata,
    StructuralTypeMode as BytecodeStructuralTypeMode,
    StructuralValueCategory as BytecodeStructuralValueCategory,
    StructuralVariantLayout as BytecodeStructuralVariantLayout, UniqueValueKind,
    VariantFieldId as BytecodeVariantFieldId, VariantId as BytecodeVariantId,
};
use lkjscript_ir::{
    BlockId, BytecodeBlockLink, BytecodeInstructionLink, BytecodeLinkMetadata, CallTarget,
    Constant, DropGlueIdentity, FailureCleanupAction as SsaFailureCleanupAction, Function,
    FunctionBytecodeLink, FunctionId, Instruction, InstructionKind, RuntimeOp, SsaType,
    StructuralDropGlueIdentity, StructuralValueCategory, Terminator, ValueId, VerifiedProgram,
};

mod constants;
mod control;
mod emit_constants;
mod emit_control;
mod emit_values;
mod enums;
mod interference;
mod locals;
mod model;
mod program;
mod runtime;
mod structural;

pub(in crate::codegen) use constants::*;
pub(in crate::codegen) use control::*;
pub(in crate::codegen) use enums::*;
pub(in crate::codegen) use interference::*;
pub(in crate::codegen) use locals::*;
pub(in crate::codegen) use model::*;
pub(crate) use program::compile_program;
pub(in crate::codegen) use runtime::*;
pub(in crate::codegen) use structural::*;
