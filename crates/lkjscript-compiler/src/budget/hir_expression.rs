use lkjscript_core::{Error, ResourceCategory, Result};

use super::{checked_add, count_usize, hir::HirCharges, type_charge::measure_type};
use crate::hir::{Expr, ExprKind, Program};

pub(super) fn measure_expressions(program: &Program, charges: &mut HirCharges) -> Result<()> {
    let mut expressions = Vec::new();
    expressions
        .try_reserve(crate::ownership::OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES)
        .map_err(|_| Error::msg("cannot reserve bounded HIR accounting stack"))?;
    expressions.push(&program.main.body);
    for function in &program.functions {
        expressions.push(&function.body);
        checked_add(
            &mut charges.traits,
            count_usize(ResourceCategory::TraitWork, function.bounds.len())?,
            ResourceCategory::TraitWork,
        )?;
        checked_add(
            &mut charges.ownership_retained,
            u64::from(function.local_count),
            ResourceCategory::OwnershipRetainedState,
        )?;
        checked_add(
            &mut charges.ownership_retained,
            count_usize(
                ResourceCategory::OwnershipRetainedState,
                function.param_places.len(),
            )?,
            ResourceCategory::OwnershipRetainedState,
        )?;
    }
    while let Some(expression) = expressions.pop() {
        checked_add(
            &mut charges.expressions,
            1,
            ResourceCategory::HirExpressions,
        )?;
        checked_add(
            &mut charges.ownership_expressions,
            1,
            ResourceCategory::OwnershipExpressions,
        )?;
        measure_type(&expression.ty, &mut charges.types)?;
        push_children(expression, &mut expressions, charges)?;
    }
    Ok(())
}

fn push_children<'a>(
    expression: &'a Expr,
    stack: &mut Vec<&'a Expr>,
    charges: &mut HirCharges,
) -> Result<()> {
    match &expression.kind {
        ExprKind::Call {
            args,
            instantiation,
            ..
        } => {
            stack.extend(args);
            if let Some(instantiation) = instantiation {
                checked_add(
                    &mut charges.traits,
                    count_usize(ResourceCategory::TraitWork, instantiation.witnesses.len())?,
                    ResourceCategory::TraitWork,
                )?;
                for substitution in &instantiation.substitutions {
                    measure_type(&substitution.ty, &mut charges.types)?;
                }
                for witness in &instantiation.witnesses {
                    measure_type(&witness.ty, &mut charges.types)?;
                }
            }
        }
        ExprKind::Operation {
            resolved_signature,
            args,
            ..
        } => {
            measure_type(resolved_signature, &mut charges.types)?;
            stack.extend(args);
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => stack.push(value),
        ExprKind::Do(children)
        | ExprKind::ProductValue {
            fields: children, ..
        }
        | ExprKind::EnumValue {
            fields: children, ..
        } => stack.extend(children),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            stack.extend([
                condition.as_ref(),
                then_branch.as_ref(),
                else_branch.as_ref(),
            ]);
        }
        ExprKind::While {
            condition, body, ..
        } => {
            stack.push(condition);
            stack.extend(body);
        }
        ExprKind::Loop {
            result_type, body, ..
        } => {
            measure_type(result_type, &mut charges.types)?;
            stack.extend(body);
        }
        ExprKind::Let { bindings, body } => {
            stack.extend(bindings.iter().map(|binding| &binding.value));
            stack.push(body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            stack.extend([initial.as_ref(), body.as_ref()])
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            stack.extend([value.as_ref(), replacement.as_ref()]);
        }
        _ => {}
    }
    Ok(())
}
