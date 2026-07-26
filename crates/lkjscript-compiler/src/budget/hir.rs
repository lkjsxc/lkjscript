use lkjscript_core::{BudgetLedger, Error, ResourceCategory, Result};

use super::{charge, charge_usize, type_charge::charge_type};
use crate::hir::{Expr, ExprKind, Program};

/// Exact post-analysis accounting. The existing Edition 1 and ownership bounds
/// protect HIR construction; this check completes before SSA construction.
pub(crate) fn charge_hir(program: &Program, ledger: &mut BudgetLedger) -> Result<()> {
    charge_usize(
        ledger,
        ResourceCategory::HirFunctions,
        program.functions.len(),
    )?;
    charge(ledger, ResourceCategory::HirFunctions, 1)?;
    for product in &program.products {
        charge_usize(
            ledger,
            ResourceCategory::ProductFields,
            product.fields.len(),
        )?;
        for field in &product.fields {
            charge_type(&field.ty, ledger)?;
        }
    }
    for definition in &program.enums {
        for field in definition
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
        {
            charge_type(&field.ty, ledger)?;
        }
    }
    for binding in &program.bindings {
        charge_type(&binding.ty, ledger)?;
    }
    charge_type(&program.main.return_type, ledger)?;
    charge_usize(
        ledger,
        ResourceCategory::OwnershipRetainedState,
        usize::from(program.main.local_count),
    )?;
    charge_usize(ledger, ResourceCategory::TraitWork, program.traits.len())?;
    charge_usize(
        ledger,
        ResourceCategory::TraitWork,
        program.implementations.len(),
    )?;

    let mut expressions = Vec::new();
    expressions
        .try_reserve(crate::ownership::OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES)
        .map_err(|_| Error::msg("cannot reserve bounded HIR accounting stack"))?;
    expressions.push(&program.main.body);
    for function in &program.functions {
        expressions.push(&function.body);
        charge_usize(ledger, ResourceCategory::TraitWork, function.bounds.len())?;
        charge_usize(
            ledger,
            ResourceCategory::OwnershipRetainedState,
            usize::from(function.local_count),
        )?;
        charge_usize(
            ledger,
            ResourceCategory::OwnershipRetainedState,
            function.param_places.len(),
        )?;
    }
    while let Some(expression) = expressions.pop() {
        charge(ledger, ResourceCategory::HirExpressions, 1)?;
        charge(ledger, ResourceCategory::OwnershipExpressions, 1)?;
        charge_type(&expression.ty, ledger)?;
        push_expression_children(expression, &mut expressions, ledger)?;
    }
    Ok(())
}

fn push_expression_children<'a>(
    expression: &'a Expr,
    stack: &mut Vec<&'a Expr>,
    ledger: &mut BudgetLedger,
) -> Result<()> {
    match &expression.kind {
        ExprKind::Call {
            args,
            instantiation,
            ..
        } => {
            stack.extend(args);
            if let Some(instantiation) = instantiation {
                charge_usize(
                    ledger,
                    ResourceCategory::TraitWork,
                    instantiation.witnesses.len(),
                )?;
                for substitution in &instantiation.substitutions {
                    charge_type(&substitution.ty, ledger)?;
                }
                for witness in &instantiation.witnesses {
                    charge_type(&witness.ty, ledger)?;
                }
            }
        }
        ExprKind::Operation {
            resolved_signature,
            args,
            ..
        } => {
            charge_type(resolved_signature, ledger)?;
            stack.extend(args);
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => stack.push(value),
        ExprKind::Do(children) => stack.extend(children),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => stack.extend([
            condition.as_ref(),
            then_branch.as_ref(),
            else_branch.as_ref(),
        ]),
        ExprKind::While {
            condition, body, ..
        } => {
            stack.push(condition);
            stack.extend(body);
        }
        ExprKind::Loop {
            result_type, body, ..
        } => {
            charge_type(result_type, ledger)?;
            stack.extend(body);
        }
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value } => stack.push(value),
        ExprKind::Let { bindings, body } => {
            stack.extend(bindings.iter().map(|binding| &binding.value));
            stack.push(body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            stack.extend([initial.as_ref(), body.as_ref()]);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            stack.push(value)
        }
        ExprKind::ProductValue { fields, .. } => stack.extend(fields),
        ExprKind::WithProductField {
            value, replacement, ..
        } => stack.extend([value.as_ref(), replacement.as_ref()]),
        _ => {}
    }
    Ok(())
}
