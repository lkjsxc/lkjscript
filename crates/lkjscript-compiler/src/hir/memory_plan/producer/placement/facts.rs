use super::*;

pub(super) struct PlacementFact<'a> {
    pub(super) id: MemoryExpressionId,
    pub(super) function: MemoryFunctionId,
    pub(super) expression: &'a Expr,
    pub(super) parent: Option<MemoryExpressionId>,
    pub(super) child_index: u32,
}

pub(super) fn collect_placement_facts(program: &hir::Program) -> Result<Vec<PlacementFact<'_>>> {
    let mut output = Vec::new();
    for (index, function) in program.functions.iter().enumerate() {
        walk_placement(
            &function.body,
            MemoryFunctionId::new(index_u32(index)?),
            None,
            0,
            &mut output,
        )?;
    }
    walk_placement(
        &program.main.body,
        MemoryFunctionId::new(index_u32(program.functions.len())?),
        None,
        0,
        &mut output,
    )?;
    Ok(output)
}

fn walk_placement<'a>(
    expression: &'a Expr,
    function: MemoryFunctionId,
    parent: Option<MemoryExpressionId>,
    child_index: u32,
    output: &mut Vec<PlacementFact<'a>>,
) -> Result<()> {
    crate::stack::grow(|| walk_placement_inner(expression, function, parent, child_index, output))
}

fn walk_placement_inner<'a>(
    expression: &'a Expr,
    function: MemoryFunctionId,
    parent: Option<MemoryExpressionId>,
    child_index: u32,
    output: &mut Vec<PlacementFact<'a>>,
) -> Result<()> {
    let id = MemoryExpressionId::new(index_u32(output.len())?);
    output.push(PlacementFact {
        id,
        function,
        expression,
        parent,
        child_index,
    });
    for (index, child) in placement_children(expression).into_iter().enumerate() {
        walk_placement(child, function, Some(id), index_u32(index)?, output)?;
    }
    Ok(())
}

fn placement_children(expression: &Expr) -> Vec<&Expr> {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => args.iter().collect(),
        ExprKind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref())
            .chain(body.iter())
            .collect(),
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
        | ExprKind::EnumUnwrap { value, .. } => vec![value],
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            vec![condition, then_branch, else_branch]
        }
        ExprKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        ExprKind::MutableLocal { initial, body, .. } => vec![initial, body],
        ExprKind::WithProductField {
            value, replacement, ..
        } => vec![value, replacement],
        _ => Vec::new(),
    }
}

pub(super) fn expression_binding(
    facts: &[PlacementFact<'_>],
    fact: &PlacementFact<'_>,
) -> Option<BindingId> {
    match &fact.expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        }
        | ExprKind::Borrow {
            binding: reference, ..
        }
        | ExprKind::BorrowBytes {
            binding: reference, ..
        } => Some(reference.binding),
        _ => local_binding(facts, fact),
    }
}

fn local_binding(facts: &[PlacementFact<'_>], fact: &PlacementFact<'_>) -> Option<BindingId> {
    let parent_id = fact.parent?;
    let parent = parent_id
        .index()
        .and_then(|index| facts.get(index))
        .filter(|candidate| candidate.id == parent_id)?;
    let ExprKind::Let { bindings, .. } = &parent.expression.kind else {
        return None;
    };
    bindings
        .get(usize::try_from(fact.child_index).ok()?)
        .map(|binding| binding.binding)
}
