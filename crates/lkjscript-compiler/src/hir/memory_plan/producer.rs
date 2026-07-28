use std::collections::{BTreeMap, HashMap};

use lkjscript_core::{Error, ResourceKind, Result};

use crate::hir::{
    self, BindingId, BindingStorage, BorrowKind, Expr, ExprKind, LocalDefinition, Operation, Type,
};

use super::{
    compute_plan_id, FunctionMemoryPlan, FunctionMemorySignature, HirMemoryPlan, MemoryAliasing,
    MemoryAllocationFailure, MemoryBindingStorage, MemoryBorrowKind, MemoryCallId, MemoryCallPlan,
    MemoryCallTarget, MemoryConstantId, MemoryConstantPlan, MemoryConstantValue, MemoryContention,
    MemoryDestruction, MemoryDropClass, MemoryDropGlueId, MemoryDropGlueKind, MemoryDropGluePlan,
    MemoryEntryId, MemoryEscape, MemoryExpressionId, MemoryExpressionKind, MemoryFunctionId,
    MemoryIdentity, MemoryLoanPlan, MemoryMode, MemoryMultiplicity, MemoryObligation,
    MemoryObligationId, MemoryObligationKind, MemoryOrigin, MemoryParameterMode, MemoryPlanEntry,
    MemoryPlanId, MemoryPlanWork, MemoryPortability, MemoryResultMode, MemoryStorage,
    MemorySubject, MemoryType, MemoryUse, MemoryUseId, MemoryUseKind, HIR_MEMORY_PLAN_SCHEMA,
    MAX_MEMORY_PLAN_CALLS, MAX_MEMORY_PLAN_CONSTANTS, MAX_MEMORY_PLAN_ENTRIES,
    MAX_MEMORY_PLAN_EXPRESSIONS, MAX_MEMORY_PLAN_FUNCTIONS, MAX_MEMORY_PLAN_LOANS,
    MAX_MEMORY_PLAN_OBLIGATIONS, MAX_MEMORY_PLAN_USES,
};

pub(super) fn derive(program: &hir::Program) -> Result<HirMemoryPlan> {
    Producer::new(program)?.run()
}

struct Producer<'a> {
    program: &'a hir::Program,
    function_ids: HashMap<BindingId, MemoryFunctionId>,
    signatures: Vec<FunctionMemorySignature>,
    functions: Vec<FunctionMemoryPlan>,
    entries: Vec<MemoryPlanEntry>,
    uses: Vec<MemoryUse>,
    loans: Vec<MemoryLoanPlan>,
    constants: Vec<MemoryConstantPlan>,
    calls: Vec<MemoryCallPlan>,
    obligations: Vec<MemoryObligation>,
    current_function: MemoryFunctionId,
    next_expression: u32,
    next_place: u32,
    expression_parents: BTreeMap<MemoryExpressionId, Option<MemoryExpressionId>>,
    work: MemoryPlanWork,
}

include!("producer/impl_00.rs");
include!("producer/impl_01.rs");
include!("producer/impl_02.rs");
include!("producer/impl_03.rs");
include!("producer/impl_04.rs");
include!("producer/impl_05.rs");
include!("producer/helpers_00.rs");
include!("producer/helpers_01.rs");
include!("producer/helpers_02.rs");
include!("producer/helpers_03.rs");
include!("producer/helpers_04.rs");
