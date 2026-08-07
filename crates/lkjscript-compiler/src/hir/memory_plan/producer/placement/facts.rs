use super::*;

pub(super) struct PlacementFact<'a> {
    pub(super) id: MemoryExpressionId,
    pub(super) function: MemoryFunctionId,
    pub(super) expression: &'a Expr,
    pub(super) parent: Option<MemoryExpressionId>,
    pub(super) child_index: u64,
}

pub(super) fn collect_placement_facts(program: &hir::Program) -> Result<Vec<PlacementFact<'_>>> {
    let mut output = Vec::new();
    for (index, function) in program.functions.iter().enumerate() {
        walk_placement(
            &function.body,
            MemoryFunctionId::new(index_u64(index)?),
            &mut output,
        )?;
    }
    walk_placement(
        &program.main.body,
        MemoryFunctionId::new(index_u64(program.functions.len())?),
        &mut output,
    )?;
    Ok(output)
}

fn walk_placement<'a>(
    root: &'a Expr,
    function: MemoryFunctionId,
    output: &mut Vec<PlacementFact<'a>>,
) -> Result<()> {
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| Error::host("placement fact work stack allocation failed"))?;
    work.push((root, None, 0_u64));
    while let Some((expression, parent, child_index)) = work.pop() {
        let id = MemoryExpressionId::new(index_u64(output.len())?);
        output
            .try_reserve(1)
            .map_err(|_| Error::host("placement fact allocation failed"))?;
        output.push(PlacementFact {
            id,
            function,
            expression,
            parent,
            child_index,
        });
        let children = placement_children(expression)?;
        work.try_reserve(children.len())
            .map_err(|_| Error::host("placement fact work stack allocation failed"))?;
        for (index, child) in children.into_iter().enumerate().rev() {
            work.push((child, Some(id), index_u64(index)?));
        }
    }
    Ok(())
}

fn placement_children(expression: &Expr) -> Result<Vec<&Expr>> {
    let mut children = Vec::new();
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => {
            children
                .try_reserve(args.len())
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.extend(args);
        }
        ExprKind::While {
            condition, body, ..
        } => {
            let count = body
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::host("placement child count overflow"))?;
            children
                .try_reserve(count)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.push(condition);
            children.extend(body);
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
        | ExprKind::EnumUnwrap { value, .. } => {
            children
                .try_reserve(1)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.push(value);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            children
                .try_reserve(3)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.extend([
                condition.as_ref(),
                then_branch.as_ref(),
                else_branch.as_ref(),
            ]);
        }
        ExprKind::Let { bindings, body } => {
            let count = bindings
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::host("placement child count overflow"))?;
            children
                .try_reserve(count)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.extend(bindings.iter().map(|binding| &binding.value));
            children.push(body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            children
                .try_reserve(2)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.extend([initial.as_ref(), body.as_ref()]);
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            children
                .try_reserve(2)
                .map_err(|_| Error::host("placement child allocation failed"))?;
            children.extend([value.as_ref(), replacement.as_ref()]);
        }
        _ => {}
    }
    Ok(children)
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
