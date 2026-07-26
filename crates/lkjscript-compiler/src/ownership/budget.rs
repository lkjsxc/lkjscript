use crate::ownership::*;

pub(in crate::ownership) fn enforce_program_budget(program: &Program) -> Result<()> {
    let mut nodes = 0usize;
    for function in &program.functions {
        charge_expression_nodes(&function.body, &mut nodes)?;
    }
    charge_expression_nodes(&program.main.body, &mut nodes)
}

pub(in crate::ownership) fn charge_expression_nodes(
    expression: &Expr,
    nodes: &mut usize,
) -> Result<()> {
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
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => charge_expression_nodes(value, nodes)?,
        ExprKind::While {
            condition, body, ..
        } => {
            charge_expression_nodes(condition, nodes)?;
            for child in body {
                charge_expression_nodes(child, nodes)?;
            }
        }
        ExprKind::Loop { body, .. } => {
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
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. } => {
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
