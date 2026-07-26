use std::collections::BTreeSet;

use super::{charges, plan::flatten_plan, usefulness::Usefulness};
use crate::analyze::*;

pub(crate) fn verify_match_plans(program: &hir::Program) -> Result<()> {
    let mut locals = BTreeSet::new();
    for (index, plan) in program.match_plans.iter().enumerate() {
        let mut places = BTreeSet::new();
        let expected_id =
            u32::try_from(index).map_err(|_| Error::msg("match plan index exceeds u32"))?;
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
            &mut locals,
            &mut places,
        )?;
        let patterns: Vec<_> = plan.arms.iter().map(|arm| arm.pattern.clone()).collect();
        let charges = charges::plan(&patterns, plan.arms.len())?;
        if plan.charges != charges {
            return Err(Error::msg("match plan logical charges are stale"));
        }
        let mut matrix = Vec::with_capacity(plan.arms.len());
        for (arm_index, arm) in plan.arms.iter().enumerate() {
            let arm_id =
                u16::try_from(arm_index).map_err(|_| Error::msg("match arm index exceeds u16"))?;
            if arm.id != arm_id || arm.body_type != plan.result_type {
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
            let mut useful = Usefulness::new(
                &program.enums,
                &program.products,
                charges.specialization_work,
            );
            if useful
                .useful(
                    &matrix,
                    std::slice::from_ref(&arm.pattern),
                    std::slice::from_ref(&plan.scrutinee.ty),
                )?
                .is_none()
            {
                return Err(Error::msg("match plan contains a stale useless arm"));
            }
            matrix.push(vec![arm.pattern.clone()]);
        }
        verify_exhaustive(program, plan, &matrix, charges.specialization_work)?;
        let (tests, projections, bindings) = flatten_plan(&plan.arms);
        if plan.tests != tests || plan.projections != projections || plan.bindings != bindings {
            return Err(Error::msg(
                "match plan tests, active projections, or bindings are stale",
            ));
        }
        let mut edges = (1..plan.arms.len())
            .map(|item| MatchEdgeTarget::Arm(u16::try_from(item).unwrap_or(u16::MAX)))
            .collect::<Vec<_>>();
        edges.extend([MatchEdgeTarget::Default, MatchEdgeTarget::Unreachable]);
        if plan.edges != edges {
            return Err(Error::msg("match plan default/unreachable edges are stale"));
        }
    }
    super::verify_markers::verify(program)
}

fn verify_exhaustive(
    program: &hir::Program,
    plan: &MatchPlan,
    matrix: &[Vec<MatchPattern>],
    work: u64,
) -> Result<()> {
    let mut useful = Usefulness::new(&program.enums, &program.products, work);
    if useful
        .useful(
            matrix,
            &[MatchPattern::Wildcard {
                ty: plan.scrutinee.ty.clone(),
            }],
            std::slice::from_ref(&plan.scrutinee.ty),
        )?
        .is_some()
    {
        Err(Error::msg("match plan exhaustive fact is stale"))
    } else {
        Ok(())
    }
}
