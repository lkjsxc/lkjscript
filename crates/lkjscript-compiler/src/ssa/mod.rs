//! Lower resolved typed HIR into backend-independent typed SSA.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use lkjscript_core::{BudgetLedger, Error, Result};
use lkjscript_ir::{
    verify, BindingId as SsaBindingId, Block, BlockId, BlockMetadata, BlockParameter,
    BorrowKind as SsaBorrowKind, CallTarget, Constant, DropEventKind, DropGlueIdentity, EffectSet,
    EnumFieldMetadata, EnumLayoutFacts, EnumMetadata, EnumVariantMetadata, FailureBehavior,
    FailureCleanupAction, FailureCleanupId, FailureCleanupPlan, FrameLocal, FrameState, Function,
    FunctionId, GenericInstantiation, ImplId, ImplMetadata, Instruction, InstructionKind,
    InstructionMetadata, LoanId as SsaLoanId, Origin, PlaceId as SsaPlaceId, PlaceMetadata,
    ProductField, ProductId, ProductMetadata, Program, RuntimeOp, Safepoint, Signature,
    SourceMetadata, SsaType, Terminator, TraitBound, TraitId, TraitMetadata, TraitRole,
    TraitWitness, TraitWitnessKind, TypeSubstitution, ValueId, VerifiedProgram,
};

use crate::hir::{self, BindingId, BindingStorage, Expr, ExprKind, LocalDefinition, Operation};
use crate::memory_plan::{
    HirMemoryPlan, MemoryDropClass, MemoryDropGlueKind, MemoryExpressionId, MemoryFunctionId,
    MemoryObligationKind, MemorySubject, MemoryVerifiedHir,
};
use crate::types::Type;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SsaMetrics {
    pub construction: Duration,
    pub verification: Duration,
    pub normalization: Duration,
}

#[cfg(test)]
pub(crate) fn lower_program(program: &hir::Program) -> Result<VerifiedProgram> {
    let memory_verified = crate::memory_plan::verify_hir_memory(program)?;
    lower_program_with_metrics(&memory_verified).map(|(program, _)| program)
}

pub(crate) fn lower_program_with_budget(
    program: &MemoryVerifiedHir<'_>,
    ledger: &mut BudgetLedger,
) -> Result<VerifiedProgram> {
    lower_program_with_metrics_and_budget(program, ledger).map(|(program, _)| program)
}

pub(crate) fn lower_program_with_metrics_and_budget(
    program: &MemoryVerifiedHir<'_>,
    ledger: &mut BudgetLedger,
) -> Result<(VerifiedProgram, SsaMetrics)> {
    let (program, metrics) = lower_program_with_metrics(program)?;
    crate::budget::reserve_bytecode_input(&program, ledger)?;
    Ok((program, metrics))
}

pub(crate) fn lower_program_with_metrics(
    memory_verified: &MemoryVerifiedHir<'_>,
) -> Result<(VerifiedProgram, SsaMetrics)> {
    let program = memory_verified.hir();
    let construction_started = Instant::now();
    crate::analyze::verify_match_plans(program)?;
    let ssa = construct_program(program, memory_verified.plan())?;
    let construction = construction_started.elapsed();
    let verification_started = Instant::now();
    let verified = verify(ssa).map_err(ir_error)?;
    let verification = verification_started.elapsed();
    let normalization_started = Instant::now();
    let normalized = lkjscript_ir::normalize_baseline(&verified).map_err(ir_error)?;
    let normalization = normalization_started.elapsed();
    Ok((
        normalized,
        SsaMetrics {
            construction,
            verification,
            normalization,
        },
    ))
}

mod builder;
mod entry_function;
mod enums;
mod facts;
mod lowering;
mod model;
mod operations;
mod program;

use enums::*;
use facts::*;
use model::*;
use operations::*;
use program::construct_program;

pub(in crate::ssa) struct PendingBlock {
    pub(in crate::ssa) id: BlockId,
    pub(in crate::ssa) parameters: Vec<BlockParameter>,
    pub(in crate::ssa) instructions: Vec<Instruction>,
    pub(in crate::ssa) terminator: Option<Terminator>,
    pub(in crate::ssa) metadata: BlockMetadata,
}

#[derive(Clone)]
pub(in crate::ssa) struct LoopTarget {
    pub(in crate::ssa) id: hir::LoopId,
    pub(in crate::ssa) header: BlockId,
    pub(in crate::ssa) exit: BlockId,
    pub(in crate::ssa) bindings: Vec<BindingId>,
    pub(in crate::ssa) active_place_bindings: Vec<BindingId>,
}

pub(in crate::ssa) struct FunctionBuilder<'a> {
    pub(in crate::ssa) product_ids: &'a HashMap<String, ProductId>,
    pub(in crate::ssa) function_ids: &'a HashMap<BindingId, FunctionId>,
    pub(in crate::ssa) function_effects: &'a HashMap<FunctionId, EffectSet>,
    pub(in crate::ssa) id: FunctionId,
    pub(in crate::ssa) name: String,
    pub(in crate::ssa) signature: Signature,
    pub(in crate::ssa) function_effect: EffectSet,
    pub(in crate::ssa) function_origin: Origin,
    pub(in crate::ssa) entry: BlockId,
    pub(in crate::ssa) blocks: Vec<PendingBlock>,
    pub(in crate::ssa) current: Option<BlockId>,
    pub(in crate::ssa) next_value: u32,
    pub(in crate::ssa) next_position: u32,
    pub(in crate::ssa) value_types: Vec<SsaType>,
    pub(in crate::ssa) places: Vec<PlaceMetadata>,
    pub(in crate::ssa) failure_cleanups: Vec<FailureCleanupPlan>,
    pub(in crate::ssa) cleanup: CleanupPlan,
    pub(in crate::ssa) active_place_bindings: Vec<BindingId>,
    pub(in crate::ssa) active_loans: BTreeMap<SsaLoanId, ActiveLoan>,
    pub(in crate::ssa) unplaced_owners: Vec<ValueId>,
    pub(in crate::ssa) env: BTreeMap<BindingId, ValueId>,
    pub(in crate::ssa) slots: BTreeMap<BindingId, u16>,
    pub(in crate::ssa) loops: Vec<LoopTarget>,
}
