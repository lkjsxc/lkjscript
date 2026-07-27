use serde::{Deserialize, Serialize};

use super::super::{Expression, TypeExpression};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatchExpressionArm {
    pub pattern: MatchPatternExpression,
    pub body: Expression,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatchPatternField {
    pub name: String,
    pub pattern: MatchPatternExpression,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum MatchPatternExpression {
    Wildcard {},
    Binding {
        name: String,
    },
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    Variant {
        value_type: TypeExpression,
        variant: String,
        fields: Vec<MatchPatternField>,
    },
    Product {
        value_type: TypeExpression,
        fields: Vec<MatchPatternField>,
    },
}

pub(super) fn measure_arms(
    arms: &[MatchExpressionArm],
    depth: u32,
    counts: &mut super::ExpressionCounts,
) {
    for arm in arms {
        measure_pattern(&arm.pattern, depth, counts);
        arm.body.measure(depth, counts);
    }
}

fn measure_pattern(
    pattern: &MatchPatternExpression,
    depth: u32,
    counts: &mut super::ExpressionCounts,
) {
    counts.nodes = counts.nodes.saturating_add(1);
    counts.depth = counts.depth.max(depth);
    let next = depth.saturating_add(1);
    match pattern {
        MatchPatternExpression::Binding { name } => add_string(counts, name),
        MatchPatternExpression::Variant {
            value_type,
            variant,
            fields,
        } => {
            value_type.measure(next, counts);
            add_string(counts, variant);
            measure_fields(fields, next, counts);
        }
        MatchPatternExpression::Product { value_type, fields } => {
            value_type.measure(next, counts);
            measure_fields(fields, next, counts);
        }
        _ => {}
    }
}

fn measure_fields(fields: &[MatchPatternField], depth: u32, counts: &mut super::ExpressionCounts) {
    for field in fields {
        add_string(counts, &field.name);
        measure_pattern(&field.pattern, depth, counts);
    }
}

fn add_string(counts: &mut super::ExpressionCounts, value: &str) {
    counts.string_bytes = counts.string_bytes.saturating_add(value.len() as u64);
}
