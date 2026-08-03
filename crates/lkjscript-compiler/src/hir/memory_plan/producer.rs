use std::collections::{BTreeMap, HashMap};

use lkjscript_core::{Error, ResourceKind, Result};

use crate::hir::{
    self, BindingId, BindingStorage, BorrowKind, Expr, ExprKind, LocalDefinition, Operation, Type,
};

use super::*;

pub(super) fn derive(program: &hir::Program) -> Result<HirMemoryPlan> {
    Producer::new(program)?.run()
}

struct Producer<'a> {
    program: &'a hir::Program,
    type_planner: TypePlanner<'a>,
    function_ids: HashMap<BindingId, MemoryFunctionId>,
    signatures: Vec<FunctionMemorySignature>,
    functions: Vec<FunctionMemoryPlan>,
    entries: Vec<MemoryPlanEntry>,
    uses: Vec<MemoryUse>,
    loans: Vec<MemoryLoanPlan>,
    constants: Vec<MemoryConstantPlan>,
    calls: Vec<MemoryCallPlan>,
    obligations: Vec<MemoryObligation>,
    destinations: Vec<MemoryDestinationPlan>,
    borrow_scopes: Vec<MemoryBorrowScopePlan>,
    current_function: MemoryFunctionId,
    next_expression: u32,
    next_place: u32,
    expression_parents: BTreeMap<MemoryExpressionId, Option<MemoryExpressionId>>,
    work: MemoryPlanWork,
}

include!("producer/type_graph.rs");
include!("producer/type_plan/mod.rs");
include!("producer/type_plan/transport.rs");
include!("producer/type_plan/transport_calls.rs");
include!("producer/type_helpers.rs");
#[path = "producer/placement/mod.rs"]
mod placement;
use placement::derive_value_placements;
include!("producer/recursive.rs");
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
