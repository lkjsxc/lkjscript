use crate::analyze::*;

pub(super) fn flatten_plan(
    arms: &[PlannedMatchArm],
) -> (
    Vec<MatchTest>,
    Vec<MatchProjection>,
    Vec<MatchBindingAssignment>,
) {
    let mut tests = Vec::new();
    let mut projections = Vec::new();
    let mut bindings = Vec::new();
    for arm in arms {
        flatten_pattern(
            arm.id,
            &arm.pattern,
            &mut Vec::new(),
            None,
            &mut tests,
            &mut projections,
            &mut bindings,
        );
    }
    (tests, projections, bindings)
}

fn flatten_pattern(
    arm: u16,
    pattern: &MatchPattern,
    path: &mut Vec<u16>,
    active: Option<VariantId>,
    tests: &mut Vec<MatchTest>,
    projections: &mut Vec<MatchProjection>,
    bindings: &mut Vec<MatchBindingAssignment>,
) {
    match pattern {
        MatchPattern::Wildcard { .. } => {}
        MatchPattern::Binding { local } => bindings.push(MatchBindingAssignment {
            arm,
            path: path.clone(),
            local: local.clone(),
        }),
        MatchPattern::Bool(value) => tests.push(MatchTest {
            arm,
            path: path.clone(),
            kind: MatchTestKind::Bool(*value),
        }),
        MatchPattern::I64(value) => tests.push(MatchTest {
            arm,
            path: path.clone(),
            kind: MatchTestKind::I64(*value),
        }),
        MatchPattern::Variant {
            enum_id,
            variant,
            layout,
            fields,
            ..
        } => {
            tests.push(MatchTest {
                arm,
                path: path.clone(),
                kind: MatchTestKind::Variant {
                    enum_id: *enum_id,
                    variant: *variant,
                    layout: *layout,
                },
            });
            flatten_fields(
                arm,
                fields,
                path,
                Some(*variant),
                tests,
                projections,
                bindings,
            );
        }
        MatchPattern::Product { fields, .. } => {
            flatten_fields(arm, fields, path, active, tests, projections, bindings)
        }
    }
}

fn flatten_fields(
    arm: u16,
    fields: &[MatchFieldPattern],
    path: &mut Vec<u16>,
    active: Option<VariantId>,
    tests: &mut Vec<MatchTest>,
    projections: &mut Vec<MatchProjection>,
    bindings: &mut Vec<MatchBindingAssignment>,
) {
    for (index, field) in fields.iter().enumerate() {
        let Ok(index) = u16::try_from(index) else {
            return;
        };
        path.push(index);
        projections.push(MatchProjection {
            arm,
            path: path.clone(),
            local: field.projection.clone(),
            active_variant: active,
        });
        flatten_pattern(
            arm,
            &field.pattern,
            path,
            active,
            tests,
            projections,
            bindings,
        );
        path.pop();
    }
}
