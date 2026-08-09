use crate::analyze::*;

pub(super) fn verify(program: &hir::Program) -> Result<()> {
    let mut counts = vec![(0_u64, 0_u64); program.match_plans.len()];
    for root in program
        .functions
        .iter()
        .map(|function| &function.body)
        .chain(std::iter::once(&program.main.body))
    {
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| Error::host("match marker work allocation failed"))?;
        pending.push(root);
        while let Some(value) = pending.pop() {
            match &value.kind {
                ExprKind::Match {
                    plan,
                    scrutinee,
                    arms,
                } => {
                    let (index, planned) = lookup(program, *plan)?;
                    if value.origin != planned.origin
                        || value.ty != planned.result_type
                        || scrutinee.ty != planned.scrutinee.ty
                        || arms.len() != planned.arms.len()
                        || arms
                            .iter()
                            .zip(&planned.arms)
                            .any(|(body, arm)| body.ty != arm.body_type)
                    {
                        return Err(Error::msg(
                            "semantic match site is inconsistent with its plan",
                        ));
                    }
                    counts[index].0 = counts[index]
                        .0
                        .checked_add(1)
                        .ok_or_else(|| Error::msg("semantic match site count overflow"))?;
                }
                ExprKind::MatchUnreachable { plan } => {
                    let (index, planned) = lookup(program, *plan)?;
                    if value.ty != Type::Never || value.origin != planned.origin {
                        return Err(Error::msg(
                            "match unreachable edge has stale Never type or origin",
                        ));
                    }
                    counts[index].1 = counts[index]
                        .1
                        .checked_add(1)
                        .ok_or_else(|| Error::msg("match unreachable provenance count overflow"))?;
                }
                _ => {}
            }
            let mut allocation_failed = false;
            hir::for_each_expression_child(value, &mut |child| {
                if allocation_failed {
                    return;
                }
                if pending.try_reserve(1).is_err() {
                    allocation_failed = true;
                } else {
                    pending.push(child);
                }
            });
            if allocation_failed {
                return Err(Error::host("match marker work allocation failed"));
            }
        }
    }
    if counts
        .iter()
        .any(|(semantic, lowered)| semantic.checked_add(*lowered) != Some(1))
    {
        return Err(Error::msg(
            "match plan has missing or duplicate semantic/lowered provenance",
        ));
    }
    Ok(())
}

fn lookup(program: &hir::Program, id: MatchPlanId) -> Result<(usize, &MatchPlan)> {
    let index = id
        .index()
        .ok_or_else(|| Error::msg("match plan identity exceeds usize"))?;
    let plan = program
        .match_plans
        .get(index)
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("match expression has stale plan identity"))?;
    Ok((index, plan))
}
