//! Mandatory bounded ownership analysis for the initial `Owned Buf` safe island.

use std::collections::{BTreeMap, BTreeSet};

use lkjscript_core::{Error, Result};

use crate::hir::{BindingId, BorrowKind, Expr, ExprKind, Function, LoanId, PlaceId, Program};
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
        if is_owned(ty) {
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

fn enforce_program_budget(program: &Program) -> Result<()> {
    let mut nodes = 0usize;
    for function in &program.functions {
        charge_expression_nodes(&function.body, &mut nodes)?;
    }
    charge_expression_nodes(&program.main.body, &mut nodes)
}

fn charge_expression_nodes(expression: &Expr, nodes: &mut usize) -> Result<()> {
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::msg("ownership analysis expression budget overflow"))?;
    if *nodes > OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES {
        return Err(Error::msg(format!(
            "ownership analysis expression budget exceeded {OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES}"
        )));
    }
    match &expression.kind {
        ExprKind::Call { args, .. } | ExprKind::Operation { args, .. } | ExprKind::Do(args) => {
            for child in args {
                charge_expression_nodes(child, nodes)?;
            }
        }
        ExprKind::While { condition, body } => {
            charge_expression_nodes(condition, nodes)?;
            for child in body {
                charge_expression_nodes(child, nodes)?;
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            charge_expression_nodes(condition, nodes)?;
            charge_expression_nodes(then_branch, nodes)?;
            charge_expression_nodes(else_branch, nodes)?;
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                charge_expression_nodes(&binding.value, nodes)?;
            }
            charge_expression_nodes(body, nodes)?;
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            charge_expression_nodes(initial, nodes)?;
            charge_expression_nodes(body, nodes)?;
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            charge_expression_nodes(value, nodes)?;
        }
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                charge_expression_nodes(field, nodes)?;
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            charge_expression_nodes(value, nodes)?;
            charge_expression_nodes(replacement, nodes)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_places(expression: &Expr, output: &mut BTreeMap<BindingId, PlaceId>) {
    match &expression.kind {
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                output.insert(binding.binding, binding.place);
                collect_places(&binding.value, output);
            }
            collect_places(body, output);
        }
        ExprKind::MutableLocal {
            binding,
            place,
            initial,
            body,
            ..
        } => {
            output.insert(*binding, *place);
            collect_places(initial, output);
            collect_places(body, output);
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for item in args {
                collect_places(item, output);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_places(condition, output);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_places(condition, output);
            collect_places(then_branch, output);
            collect_places(else_branch, output);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            collect_places(value, output);
        }
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                collect_places(field, output);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_places(value, output);
            collect_places(replacement, output);
        }
        _ => {}
    }
}

fn check_expr(
    program: &Program,
    expression: &Expr,
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
    context: UseContext,
) -> Result<()> {
    expire_dead_loans(state, &uses(expression).union(future).copied().collect());
    reject_unsupported_type_placement(&expression.ty)?;
    match &expression.kind {
        ExprKind::Load(reference) => {
            let ty = &program
                .binding(reference.binding)
                .ok_or_else(|| Error::msg("ownership load references unknown binding"))?
                .ty;
            if is_owned(ty) {
                return Err(Error::msg(
                    "Owned Buf is affine and cannot be loaded or copied; use move/ name /move",
                ));
            }
            if is_ref_mut(ty) || is_ref(ty) {
                if context != UseContext::ExactReferenceArgument {
                    return Err(Error::msg(
                        "lexical references may be used only as exact reference arguments in this slice",
                    ));
                }
                if let Some((place, loan)) = state.reference_loans.get(&reference.binding) {
                    if !state
                        .loans
                        .get(place)
                        .is_some_and(|loans| loans.iter().any(|item| item.id == *loan))
                    {
                        return Err(Error::msg("use of lexical reference after its loan ended"));
                    }
                }
                if is_ref_mut(ty) && !state.consumed_ref_mut.insert(reference.binding) {
                    return Err(Error::msg(
                        "RefMut Buf is affine and may be used only once in this slice",
                    ));
                }
            }
        }
        ExprKind::Move { place, binding } => {
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg("move has mismatched place/binding identity"));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("use after move or double move of Owned Buf"));
            }
            if state
                .loans
                .get(place)
                .is_some_and(|loans| !loans.is_empty())
            {
                return Err(Error::msg("cannot move Owned Buf while it is borrowed"));
            }
            state.initialized.insert(*place, false);
        }
        ExprKind::Borrow {
            place,
            loan,
            kind,
            binding,
        } => {
            if !matches!(
                context,
                UseContext::ExactReferenceArgument | UseContext::DirectLetInitializer
            ) {
                return Err(Error::msg(
                    "borrow is permitted only as an exact direct reference argument or direct let initializer in the initial ownership slice",
                ));
            }
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg("borrow has mismatched place/binding identity"));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("cannot borrow Owned Buf after move"));
            }
            if state.loans.values().flatten().any(|item| item.id == *loan) {
                return Err(Error::msg("duplicate LoanId in ownership facts"));
            }
            let live = state.loans.entry(*place).or_default();
            if (*kind == BorrowKind::Mutable && !live.is_empty())
                || (*kind == BorrowKind::Shared
                    && live.iter().any(|item| item.kind == BorrowKind::Mutable))
            {
                return Err(Error::msg(
                    "conflicting shared and exclusive Owned Buf loans",
                ));
            }
            live.push(Loan {
                id: *loan,
                kind: *kind,
                binding: None,
            });
        }
        ExprKind::Call { args, .. } => {
            for argument in args {
                if is_owned(&argument.ty) && !matches!(argument.kind, ExprKind::Move { .. }) {
                    return Err(Error::msg(
                        "Owned Buf call arguments require explicit move of a whole local place",
                    ));
                }
            }
            check_arguments(program, args, places, state, future)?;
        }
        ExprKind::Operation { args, .. } => {
            check_arguments(program, args, places, state, future)?;
        }
        ExprKind::Do(expressions) => check_sequence(program, expressions, places, state, future)?,
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let branch_uses = uses(then_branch)
                .union(&uses(else_branch))
                .copied()
                .collect();
            check_expr(
                program,
                condition,
                places,
                state,
                &branch_uses,
                UseContext::Ordinary,
            )?;
            let mut left = state.clone();
            let mut right = state.clone();
            check_expr(
                program,
                then_branch,
                places,
                &mut left,
                future,
                UseContext::Ordinary,
            )?;
            check_expr(
                program,
                else_branch,
                places,
                &mut right,
                future,
                UseContext::Ordinary,
            )?;
            expire_dead_loans(&mut left, future);
            expire_dead_loans(&mut right, future);
            if left != right {
                return Err(Error::msg(
                    "ownership and loan state must match exactly at branch join",
                ));
            }
            *state = left;
        }
        ExprKind::While { condition, body } => {
            if contains_ownership_action(condition)
                || body.iter().any(contains_ownership_action)
                || uses_reference_binding(program, condition)?
                || body.iter().try_fold(false, |found, item| {
                    Ok::<bool, Error>(found || uses_reference_binding(program, item)?)
                })?
                || !state.loans.is_empty()
            {
                return Err(Error::msg(
                    "loop-carried moves or loans are unsupported in the initial ownership slice",
                ));
            }
            let before = state.clone();
            check_expr(
                program,
                condition,
                places,
                state,
                future,
                UseContext::Ordinary,
            )?;
            check_sequence(program, body, places, state, future)?;
            if *state != before {
                return Err(Error::msg(
                    "ownership initialization state must be equal after a loop iteration",
                ));
            }
        }
        ExprKind::Let { bindings, body } => {
            for (index, local) in bindings.iter().enumerate() {
                let later = uses_bindings(&bindings[index.saturating_add(1)..], body, future);
                let initializer_context = if matches!(local.value.kind, ExprKind::Borrow { .. }) {
                    UseContext::DirectLetInitializer
                } else {
                    UseContext::Ordinary
                };
                check_expr(
                    program,
                    &local.value,
                    places,
                    state,
                    &later,
                    initializer_context,
                )?;
                if is_owned(&expression_of_binding(program, local.binding)?) {
                    state.initialized.insert(local.place, true);
                }
                if let ExprKind::Borrow {
                    place, loan, kind, ..
                } = local.value.kind
                {
                    if state
                        .reference_loans
                        .insert(local.binding, (place, loan))
                        .is_some()
                    {
                        return Err(Error::msg("duplicate local reference loan binding"));
                    }
                    if let Some(item) = state
                        .loans
                        .get_mut(&place)
                        .and_then(|loans| loans.iter_mut().find(|item| item.id == loan))
                    {
                        item.binding = Some(local.binding);
                        item.kind = kind;
                    }
                }
            }
            check_expr(program, body, places, state, future, UseContext::Ordinary)?;
            for local in bindings.iter().rev() {
                end_reference_binding(state, local.binding);
                end_place_scope(state, local.place);
                state.consumed_ref_mut.remove(&local.binding);
            }
        }
        ExprKind::MutableLocal {
            binding,
            place,
            initial,
            body,
            ..
        } => {
            check_expr(
                program,
                initial,
                places,
                state,
                &uses(body),
                UseContext::Ordinary,
            )?;
            if is_owned(&expression_of_binding(program, *binding)?) {
                state.initialized.insert(*place, true);
            }
            check_expr(program, body, places, state, future, UseContext::Ordinary)?;
            end_reference_binding(state, *binding);
            end_place_scope(state, *place);
            state.consumed_ref_mut.remove(binding);
        }
        ExprKind::SetLocal { target, value, .. } => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
            let ty = expression_of_binding(program, *target)?;
            if is_owned(&ty) {
                let place = places
                    .get(target)
                    .ok_or_else(|| Error::msg("Owned Buf assignment target has no PlaceId"))?;
                if state.initialized.get(place).copied().unwrap_or(false) {
                    return Err(Error::msg(
                        "Owned Buf var assignment is only reinitialization after move in this slice",
                    ));
                }
                if state
                    .loans
                    .get(place)
                    .is_some_and(|loans| !loans.is_empty())
                {
                    return Err(Error::msg("cannot reinitialize Owned Buf while borrowed"));
                }
                state.initialized.insert(*place, true);
            }
        }
        ExprKind::ProductValue { fields, .. } => {
            check_sequence(program, fields, places, state, future)?;
        }
        ExprKind::ProductField { value, .. } => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            check_expr(
                program,
                value,
                places,
                state,
                &uses(replacement),
                UseContext::Ordinary,
            )?;
            check_expr(
                program,
                replacement,
                places,
                state,
                future,
                UseContext::Ordinary,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn check_arguments(
    program: &Program,
    args: &[Expr],
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
) -> Result<()> {
    let mut temporary = Vec::new();
    for (index, argument) in args.iter().enumerate() {
        let mut later = future.clone();
        for item in &args[index.saturating_add(1)..] {
            later.extend(uses(item));
        }
        let context = if is_ref(&argument.ty) || is_ref_mut(&argument.ty) {
            UseContext::ExactReferenceArgument
        } else {
            UseContext::Ordinary
        };
        check_expr(program, argument, places, state, &later, context)?;
        if let ExprKind::Borrow { place, loan, .. } = argument.kind {
            temporary.push((place, loan));
        }
    }
    for (place, loan) in temporary {
        end_loan(state, place, loan);
    }
    Ok(())
}

fn check_sequence(
    program: &Program,
    expressions: &[Expr],
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
) -> Result<()> {
    for (index, expression) in expressions.iter().enumerate() {
        let mut later = future.clone();
        for item in &expressions[index.saturating_add(1)..] {
            later.extend(uses(item));
        }
        check_expr(
            program,
            expression,
            places,
            state,
            &later,
            UseContext::Ordinary,
        )?;
    }
    Ok(())
}

fn expire_dead_loans(state: &mut State, live_bindings: &BTreeSet<BindingId>) {
    let dead: Vec<BindingId> = state
        .reference_loans
        .keys()
        .copied()
        .filter(|binding| !live_bindings.contains(binding))
        .collect();
    for binding in dead {
        end_reference_binding(state, binding);
    }
}

fn end_reference_binding(state: &mut State, binding: BindingId) {
    if let Some((place, loan)) = state.reference_loans.remove(&binding) {
        end_loan(state, place, loan);
    }
}

fn end_place_scope(state: &mut State, place: PlaceId) {
    state.initialized.remove(&place);
    state.loans.remove(&place);
    let references: Vec<BindingId> = state
        .reference_loans
        .iter()
        .filter_map(|(binding, (owner, _))| (*owner == place).then_some(*binding))
        .collect();
    for binding in references {
        state.reference_loans.remove(&binding);
        state.consumed_ref_mut.remove(&binding);
    }
}

fn end_loan(state: &mut State, place: PlaceId, loan: LoanId) {
    if let Some(loans) = state.loans.get_mut(&place) {
        loans.retain(|item| item.id != loan);
        if loans.is_empty() {
            state.loans.remove(&place);
        }
    }
}

fn uses(expression: &Expr) -> BTreeSet<BindingId> {
    let mut output = BTreeSet::new();
    collect_uses(expression, &mut output);
    output
}

fn collect_uses(expression: &Expr, output: &mut BTreeSet<BindingId>) {
    match &expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        } => {
            output.insert(reference.binding);
        }
        ExprKind::Borrow { binding, .. } => {
            output.insert(binding.binding);
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for item in args {
                collect_uses(item, output);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_uses(condition, output);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_uses(condition, output);
            collect_uses(then_branch, output);
            collect_uses(else_branch, output);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                collect_uses(&binding.value, output);
            }
            collect_uses(body, output);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            collect_uses(initial, output);
            collect_uses(body, output);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            collect_uses(value, output);
        }
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                collect_uses(field, output);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_uses(value, output);
            collect_uses(replacement, output);
        }
        _ => {}
    }
}

fn uses_bindings(
    bindings: &[crate::hir::LocalDefinition],
    body: &Expr,
    future: &BTreeSet<BindingId>,
) -> BTreeSet<BindingId> {
    let mut result = future.clone();
    for binding in bindings {
        result.extend(uses(&binding.value));
    }
    result.extend(uses(body));
    result
}

fn contains_ownership_action(expression: &Expr) -> bool {
    if matches!(
        expression.kind,
        ExprKind::Move { .. } | ExprKind::Borrow { .. }
    ) {
        return true;
    }
    let mut actions = false;
    walk_children(expression, &mut |child| {
        actions |= contains_ownership_action(child);
    });
    actions
}

fn walk_children(expression: &Expr, action: &mut impl FnMut(&Expr)) {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for child in args {
                action(child);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                action(condition);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            action(condition);
            action(then_branch);
            action(else_branch);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                action(&binding.value);
            }
            action(body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            action(initial);
            action(body);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => action(value),
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                action(field);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            action(value);
            action(replacement);
        }
        _ => {}
    }
}

fn reject_unsupported_type_placement(ty: &Type) -> Result<()> {
    match ty {
        Type::List(inner) | Type::Option(inner) if contains_ownership(inner) => Err(Error::msg(
            "ownership/reference values cannot be stored in List or Option",
        )),
        Type::Result(ok, error) if contains_ownership(ok) || contains_ownership(error) => Err(
            Error::msg("ownership/reference values cannot be stored in Result"),
        ),
        _ => Ok(()),
    }
}

fn contains_ownership(ty: &Type) -> bool {
    match ty {
        Type::Owned(_) | Type::Ref(_) | Type::RefMut(_) => true,
        Type::List(inner) | Type::Option(inner) => contains_ownership(inner),
        Type::Result(ok, error) => contains_ownership(ok) || contains_ownership(error),
        Type::Fn { params, ret } => {
            params.iter().any(contains_ownership) || contains_ownership(ret)
        }
        Type::Forall { body, .. } => contains_ownership(body),
        _ => false,
    }
}

fn uses_reference_binding(program: &Program, expression: &Expr) -> Result<bool> {
    for binding in uses(expression) {
        let ty = expression_of_binding(program, binding)?;
        if is_ref(&ty) || is_ref_mut(&ty) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn expression_of_binding(program: &Program, binding: BindingId) -> Result<Type> {
    program
        .binding(binding)
        .map(|binding| binding.ty.clone())
        .ok_or_else(|| Error::msg("ownership fact references unknown binding"))
}

fn is_owned(ty: &Type) -> bool {
    matches!(ty, Type::Owned(inner) if inner.as_ref() == &Type::Buf)
}

fn is_ref(ty: &Type) -> bool {
    matches!(ty, Type::Ref(inner) if inner.as_ref() == &Type::Buf)
}

fn is_ref_mut(ty: &Type) -> bool {
    matches!(ty, Type::RefMut(inner) if inner.as_ref() == &Type::Buf)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{check, OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES};
    use crate::hir::{EffectSet, Expr, ExprKind, Main, Program, SourceId, Type};

    #[test]
    fn aggregate_expression_budget_is_enforced_on_constructed_hir() {
        let origin = SourceId::new(0);
        let leaf = || Expr {
            ty: Type::Unit,
            effects: EffectSet::PURE,
            origin,
            kind: ExprKind::LitUnit,
        };
        let body = Expr {
            ty: Type::Unit,
            effects: EffectSet::PURE,
            origin,
            kind: ExprKind::Do(
                (0..OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES)
                    .map(|_| leaf())
                    .collect(),
            ),
        };
        let program = Program {
            sources: Vec::new(),
            bindings: Vec::new(),
            products: Vec::new(),
            traits: Vec::new(),
            implementations: Vec::new(),
            functions: Vec::new(),
            main: Main {
                origin,
                return_type: Type::Unit,
                local_count: 0,
                body,
            },
            global_layout: Vec::new(),
        };
        let error = check(&program)
            .expect_err("constructed HIR over the aggregate budget must fail")
            .to_string();
        assert!(
            error.contains(&format!(
                "ownership analysis expression budget exceeded {OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES}"
            )),
            "wrong aggregate budget diagnostic: {error}"
        );
    }
}
