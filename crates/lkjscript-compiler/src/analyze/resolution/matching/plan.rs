use crate::analyze::*;

use super::usefulness::reserve;

pub(super) fn flatten_plan(
    arms: &[PlannedMatchArm],
) -> Result<(
    Vec<MatchTest>,
    Vec<MatchProjection>,
    Vec<MatchBindingAssignment>,
)> {
    enum Work<'a> {
        Pattern(&'a MatchPattern, Option<VariantId>),
        Field(&'a MatchFieldPattern, usize, Option<VariantId>),
        PopPath,
    }

    let mut tests = Vec::new();
    let mut projections = Vec::new();
    let mut bindings = Vec::new();
    let mut path = Vec::new();
    let mut work = Vec::new();
    for arm in arms {
        reserve(&mut work, 1, "match plan flatten work stack")?;
        work.push(Work::Pattern(&arm.pattern, None));
        while let Some(item) = work.pop() {
            match item {
                Work::Pattern(pattern, active) => {
                    match pattern {
                        MatchPattern::Wildcard { .. } => {}
                        MatchPattern::Binding { local } => {
                            reserve(&mut bindings, 1, "match binding plan")?;
                            bindings.push(MatchBindingAssignment {
                                arm: arm.id,
                                path: clone_path(&path)?,
                                local: local.clone(),
                            });
                        }
                        MatchPattern::Bool(value) => {
                            reserve(&mut tests, 1, "match test plan")?;
                            tests.push(MatchTest {
                                arm: arm.id,
                                path: clone_path(&path)?,
                                kind: MatchTestKind::Bool(*value),
                            });
                        }
                        MatchPattern::I64(value) => {
                            reserve(&mut tests, 1, "match test plan")?;
                            tests.push(MatchTest {
                                arm: arm.id,
                                path: clone_path(&path)?,
                                kind: MatchTestKind::I64(*value),
                            });
                        }
                        MatchPattern::Variant {
                            enum_id,
                            variant,
                            layout,
                            fields,
                            ..
                        } => {
                            reserve(&mut tests, 1, "match test plan")?;
                            tests.push(MatchTest {
                                arm: arm.id,
                                path: clone_path(&path)?,
                                kind: MatchTestKind::Variant {
                                    enum_id: *enum_id,
                                    variant: *variant,
                                    layout: *layout,
                                },
                            });
                            reserve(&mut work, fields.len(), "match plan flatten work stack")?;
                            work.extend(
                                fields.iter().enumerate().rev().map(|(index, field)| {
                                    Work::Field(field, index, Some(*variant))
                                }),
                            );
                        }
                        MatchPattern::Product { fields, .. } => {
                            reserve(&mut work, fields.len(), "match plan flatten work stack")?;
                            work.extend(
                                fields
                                    .iter()
                                    .enumerate()
                                    .rev()
                                    .map(|(index, field)| Work::Field(field, index, active)),
                            );
                        }
                    }
                }
                Work::Field(field, index, active) => {
                    let index = u64::try_from(index)
                        .map_err(|_| Error::host("match pattern path index exceeds u64"))?;
                    reserve(&mut path, 1, "match pattern path")?;
                    path.push(index);
                    if let Some(local) = &field.projection {
                        reserve(&mut projections, 1, "match projection plan")?;
                        projections.push(MatchProjection {
                            arm: arm.id,
                            path: clone_path(&path)?,
                            local: local.clone(),
                            active_variant: active,
                        });
                    }
                    reserve(&mut work, 2, "match plan flatten work stack")?;
                    work.push(Work::PopPath);
                    work.push(Work::Pattern(&field.pattern, active));
                }
                Work::PopPath => {
                    path.pop()
                        .ok_or_else(|| Error::msg("match plan flatten path underflow"))?;
                }
            }
        }
        if !path.is_empty() {
            return Err(Error::msg("match plan flatten left a stale field path"));
        }
    }
    Ok((tests, projections, bindings))
}

pub(super) fn build_plan(
    id: MatchPlanId,
    origin: Origin,
    scrutinee: MatchLocal,
    arms: Vec<PlannedMatchArm>,
    enums: &[EnumDefinition],
    products: &[ProductDefinition],
) -> Result<MatchPlan> {
    if arms.is_empty() {
        return Err(Error::msg("match arms must not be empty"));
    }
    let mut result_type = Type::Never;
    for arm in &arms {
        result_type = Type::join_control(&result_type, &arm.body_type).ok_or_else(|| {
            Error::msg(format!(
                "reachable match arm types must be exactly equal: {} vs {}",
                result_type, arm.body_type
            ))
        })?;
    }
    check_usefulness(&arms, &scrutinee.ty, enums, products)?;
    let (tests, projections, bindings) = flatten_plan(&arms)?;
    let edge_capacity = arms
        .len()
        .checked_add(1)
        .ok_or_else(|| Error::host("match edge count overflow"))?;
    let mut edges = Vec::new();
    edges
        .try_reserve(edge_capacity)
        .map_err(|_| Error::host("match edge allocation failed"))?;
    for index in 1..arms.len() {
        edges.push(MatchEdgeTarget::Arm(
            u64::try_from(index).map_err(|_| Error::msg("match arm index exceeds u64"))?,
        ));
    }
    edges.extend([MatchEdgeTarget::Default, MatchEdgeTarget::Unreachable]);
    Ok(MatchPlan {
        id,
        origin,
        scrutinee,
        result_type,
        arms,
        tests,
        projections,
        bindings,
        edges,
        exhaustive: true,
        witness: None,
    })
}

fn check_usefulness(
    arms: &[PlannedMatchArm],
    scrutinee: &Type,
    enums: &[EnumDefinition],
    products: &[ProductDefinition],
) -> Result<()> {
    let mut matrix = Vec::new();
    matrix
        .try_reserve(arms.len())
        .map_err(|_| Error::host("match usefulness input matrix allocation failed"))?;
    let mut usefulness = super::usefulness::Usefulness::new(enums, products)?;
    for arm in arms {
        let candidate = [&arm.pattern];
        if usefulness
            .useful(&matrix, &candidate, std::slice::from_ref(scrutinee))?
            .is_none()
        {
            return Err(Error::msg(format!(
                "useless or subsumed match arm {}",
                arm.id
            )));
        }
        matrix.push(&arm.pattern);
    }
    let wildcard = MatchPattern::Wildcard {
        ty: scrutinee.clone(),
    };
    if let Some(witness) =
        usefulness.useful(&matrix, &[&wildcard], std::slice::from_ref(scrutinee))?
    {
        let root = *witness
            .first()
            .ok_or_else(|| Error::msg("match usefulness returned an empty witness"))?;
        let rendered = usefulness.render_witness(root)?;
        return Err(Error::msg(format!(
            "nonexhaustive match; canonical typed witness: {rendered}",
        )));
    }
    Ok(())
}

fn clone_path(path: &[u64]) -> Result<Vec<u64>> {
    let mut output = Vec::new();
    reserve(&mut output, path.len(), "match plan path copy")?;
    output.extend_from_slice(path);
    Ok(output)
}
