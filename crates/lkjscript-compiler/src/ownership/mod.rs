//! Mandatory ownership analysis for the initial `byte-vector` safe island.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lkjscript_core::{Error, Result};

use crate::hir::{
    BindingId, BindingKind, BorrowKind, Expr, ExprKind, Function, LoanId, Operation, PlaceId,
    Program,
};
use crate::types::Type;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct State {
    initialized: BTreeMap<PlaceId, bool>,
    loans: BTreeMap<PlaceId, Vec<Loan>>,
    reference_loans: BTreeMap<BindingId, (PlaceId, LoanId)>,
    pinned_references: HashMap<BindingId, u64>,
    consumed_ref_mut: BTreeSet<BindingId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Loan {
    id: LoanId,
    kind: BorrowKind,
    binding: Option<BindingId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UseContext {
    Ordinary,
    ExactReferenceArgument,
    DirectLetInitializer,
}

pub(crate) fn check(program: &Program) -> Result<()> {
    for function in &program.functions {
        check_function(program, function)?;
    }
    let plan = OwnershipPlan::build(program, &program.main.body, std::iter::empty())?;
    check_body(program, &program.main.body, &plan, State::default())
}

pub(crate) fn draft_parameter_load_is_supported(ty: &Type) -> bool {
    !is_owned(ty) && !is_affine_resource(ty)
}

fn check_function(program: &Program, function: &Function) -> Result<()> {
    let plan = OwnershipPlan::build(
        program,
        &function.body,
        function
            .params
            .iter()
            .copied()
            .zip(function.param_places.iter().copied()),
    )?;
    let mut state = State::default();
    for (binding, place) in function
        .params
        .iter()
        .copied()
        .zip(function.param_places.iter().copied())
    {
        let ty = &program
            .binding(binding)
            .ok_or_else(|| Error::msg("ownership parameter references unknown binding"))?
            .ty;
        if is_owned(ty) || is_affine_resource(ty) {
            state.initialized.insert(place, true);
        }
    }
    check_body(program, &function.body, &plan, state)
}

fn check_body(
    program: &Program,
    body: &Expr,
    plan: &OwnershipPlan,
    mut state: State,
) -> Result<()> {
    let mut cursor = ExprCursor::default();
    let mut future = FutureUses::default();
    check_expr(
        program,
        body,
        plan,
        &mut cursor,
        &mut state,
        &mut future,
        UseContext::Ordinary,
    )?;
    cursor.finish(plan)
}

mod checking;
mod liveness;
mod types;

use checking::*;
use liveness::*;
use types::*;
