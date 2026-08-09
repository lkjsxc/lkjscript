use std::collections::BTreeSet;

use super::{plan::flatten_plan, usefulness::Usefulness};
use crate::analyze::*;

pub(crate) fn verify_match_plans(program: &hir::Program) -> Result<()> {
    let mut locals = BTreeSet::new();
    for (index, plan) in program.match_plans.iter().enumerate() {
        let mut places = BTreeSet::new();
        let expected_id =
            u64::try_from(index).map_err(|_| Error::msg("match plan identity exceeds u64"))?;
        if plan.id.raw() != expected_id
            || plan.arms.is_empty()
            || !plan.exhaustive
            || plan.witness.is_some()
        {
            return Err(Error::msg(
                "match plan has stale identity or exhaustive witness fact",
            ));
        }
        super::verify_pattern::local(
            program,
            plan.origin,
            &plan.scrutinee,
            BindingKind::MatchTemporary,
            &mut locals,
            &mut places,
        )?;
        let mut matrix = Vec::new();
        matrix
            .try_reserve(plan.arms.len())
            .map_err(|_| Error::host("match verifier usefulness matrix allocation failed"))?;
        let mut usefulness = Usefulness::new(&program.enums, &program.products)?;
        for (arm_index, arm) in plan.arms.iter().enumerate() {
            let arm_id = u64::try_from(arm_index)
                .map_err(|_| Error::msg("match arm identity exceeds u64"))?;
            if arm.id != arm_id
                || Type::join_control(&arm.body_type, &plan.result_type)
                    != Some(plan.result_type.clone())
            {
                return Err(Error::msg("match plan arm identity/order/join is stale"));
            }
            super::verify_pattern::pattern(
                program,
                plan.origin,
                &arm.pattern,
                &plan.scrutinee.ty,
                &mut locals,
                &mut places,
            )?;
            if usefulness
                .useful(
                    &matrix,
                    &[&arm.pattern],
                    std::slice::from_ref(&plan.scrutinee.ty),
                )?
                .is_none()
            {
                return Err(Error::msg("match plan contains a stale useless arm"));
            }
            matrix.push(&arm.pattern);
        }
        verify_exhaustive(&mut usefulness, plan, &matrix)?;
        let (tests, projections, bindings) = flatten_plan(&plan.arms)?;
        if plan.tests != tests || plan.projections != projections || plan.bindings != bindings {
            return Err(Error::msg(
                "match plan tests, active projections, or bindings are stale",
            ));
        }
        let edge_capacity = plan
            .arms
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::host("match verifier edge count overflow"))?;
        let mut edges = Vec::new();
        edges
            .try_reserve(edge_capacity)
            .map_err(|_| Error::host("match verifier edge allocation failed"))?;
        for item in 1..plan.arms.len() {
            edges.push(MatchEdgeTarget::Arm(
                u64::try_from(item).map_err(|_| Error::msg("match arm index exceeds u64"))?,
            ));
        }
        edges.extend([MatchEdgeTarget::Default, MatchEdgeTarget::Unreachable]);
        if plan.edges != edges {
            return Err(Error::msg("match plan default/unreachable edges are stale"));
        }
    }
    super::verify_markers::verify(program)
}

fn verify_exhaustive(
    usefulness: &mut Usefulness<'_>,
    plan: &MatchPlan,
    matrix: &[&MatchPattern],
) -> Result<()> {
    let wildcard = MatchPattern::Wildcard {
        ty: plan.scrutinee.ty.clone(),
    };
    if usefulness
        .useful(
            matrix,
            &[&wildcard],
            std::slice::from_ref(&plan.scrutinee.ty),
        )?
        .is_some()
    {
        Err(Error::msg("match plan exhaustive fact is stale"))
    } else {
        Ok(())
    }
}
