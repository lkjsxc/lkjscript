mod build;
mod patterns;
mod patterns_aggregate;
mod plan;
mod resolve;
mod usefulness;
mod usefulness_matrix;
mod usefulness_space;
mod verify;
mod verify_markers;
mod verify_pattern;
mod witness;

pub(crate) use verify::verify_match_plans;

pub(crate) fn build_match_plan(
    id: crate::hir::MatchPlanId,
    origin: crate::hir::Origin,
    scrutinee: crate::hir::MatchLocal,
    arms: Vec<crate::hir::PlannedMatchArm>,
    enums: &[crate::hir::EnumDefinition],
    products: &[crate::hir::ProductDefinition],
) -> lkjscript_core::Result<crate::hir::MatchPlan> {
    plan::build_plan(id, origin, scrutinee, arms, enums, products)
}

pub(crate) fn lower_semantic_matches(
    program: &mut crate::hir::Program,
) -> lkjscript_core::Result<()> {
    let plans = &program.match_plans;
    let enums = &program.enums;
    let mut lower = |id: crate::hir::MatchPlanId,
                     scrutinee: crate::hir::Expr,
                     arms: Vec<crate::hir::Expr>| {
        let plan = id
            .index()
            .and_then(|index| plans.get(index))
            .filter(|plan| plan.id == id)
            .ok_or_else(|| lkjscript_core::Error::msg("semantic match plan identity is stale"))?;
        build::lower(plan, scrutinee, arms, enums)
    };
    for function in &mut program.functions {
        function.body = function.body.try_lower_semantic_matches(&mut lower)?;
    }
    program.main.body = program.main.body.try_lower_semantic_matches(&mut lower)?;
    Ok(())
}
