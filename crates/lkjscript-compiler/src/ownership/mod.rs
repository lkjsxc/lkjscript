//! Mandatory bounded ownership analysis for the initial `byte-vector` safe island.

use std::collections::{BTreeMap, BTreeSet};

use lkjscript_core::{Error, Result};

use crate::hir::{
    BindingId, BorrowKind, Expr, ExprKind, Function, LoanId, Operation, PlaceId, Program,
};
use crate::types::Type;

pub(crate) const OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES: usize = 16_384;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct State {
    initialized: BTreeMap<PlaceId, bool>,
    loans: BTreeMap<PlaceId, Vec<Loan>>,
    reference_loans: BTreeMap<BindingId, (PlaceId, LoanId)>,
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
    enforce_program_budget(program)?;
    for function in &program.functions {
        check_function(program, function)?;
    }
    let mut places = BTreeMap::new();
    collect_places(&program.main.body, &mut places);
    let mut state = State::default();
    validate_declared_places(program, &places)?;
    check_expr(
        program,
        &program.main.body,
        &places,
        &mut state,
        &BTreeSet::new(),
        UseContext::Ordinary,
    )?;
    Ok(())
}

fn check_function(program: &Program, function: &Function) -> Result<()> {
    let mut places: BTreeMap<BindingId, PlaceId> = function
        .params
        .iter()
        .copied()
        .zip(function.param_places.iter().copied())
        .collect();
    collect_places(&function.body, &mut places);
    let mut state = State::default();
    validate_declared_places(program, &places)?;
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
    check_expr(
        program,
        &function.body,
        &places,
        &mut state,
        &BTreeSet::new(),
        UseContext::Ordinary,
    )
}

fn validate_declared_places(
    program: &Program,
    places: &BTreeMap<BindingId, PlaceId>,
) -> Result<()> {
    let mut identities = BTreeSet::new();
    for (binding, place) in places {
        if !identities.insert(*place) {
            return Err(Error::msg("ownership analysis found duplicate PlaceId"));
        }
        let _ty = &program
            .binding(*binding)
            .ok_or_else(|| Error::msg("ownership place references unknown binding"))?
            .ty;
    }
    Ok(())
}

mod budget;
mod checking;
mod places;
mod types;
mod uses;

use budget::*;
use checking::*;
use places::*;
use types::*;
use uses::*;

#[cfg(test)]
mod tests;
