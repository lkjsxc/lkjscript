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

#[derive(Debug, Serialize)]
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

#[derive(Deserialize)]
#[serde(
    remote = "MatchPatternExpression",
    tag = "kind",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum MatchPatternExpressionDef {
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

impl<'de> Deserialize<'de> for MatchPatternExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::stack::grow(|| MatchPatternExpressionDef::deserialize(deserializer))
    }
}

impl Clone for MatchPatternExpression {
    fn clone(&self) -> Self {
        crate::stack::grow(|| match self {
            Self::Wildcard {} => Self::Wildcard {},
            Self::Binding { name } => Self::Binding { name: name.clone() },
            Self::Bool { value } => Self::Bool { value: *value },
            Self::I64 { value } => Self::I64 { value: *value },
            Self::Variant {
                value_type,
                variant,
                fields,
            } => Self::Variant {
                value_type: value_type.clone(),
                variant: variant.clone(),
                fields: fields.clone(),
            },
            Self::Product { value_type, fields } => Self::Product {
                value_type: value_type.clone(),
                fields: fields.clone(),
            },
        })
    }
}

impl Drop for MatchPatternExpression {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut pattern) = pending.pop() {
            take_children(&mut pattern, &mut pending);
        }
    }
}

fn take_children(pattern: &mut MatchPatternExpression, pending: &mut Vec<MatchPatternExpression>) {
    match pattern {
        MatchPatternExpression::Variant { fields, .. }
        | MatchPatternExpression::Product { fields, .. } => {
            pending.extend(
                std::mem::take(fields)
                    .into_iter()
                    .map(|field| field.pattern),
            );
        }
        _ => {}
    }
}

pub(super) fn measure_arms(
    arms: &[MatchExpressionArm],
    depth: u64,
    counts: &mut super::ExpressionCounts,
) {
    for arm in arms {
        measure_pattern(&arm.pattern, depth, counts);
        arm.body.measure(depth, counts);
    }
}

#[allow(clippy::expect_used)]
fn measure_pattern(
    pattern: &MatchPatternExpression,
    depth: u64,
    counts: &mut super::ExpressionCounts,
) {
    counts.nodes = counts
        .nodes
        .checked_add(1)
        .expect("host-addressable pattern trees fit u64");
    counts.depth = counts.depth.max(depth);
    let next = depth
        .checked_add(1)
        .expect("host-addressable pattern depth fits u64");
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

fn measure_fields(fields: &[MatchPatternField], depth: u64, counts: &mut super::ExpressionCounts) {
    for field in fields {
        add_string(counts, &field.name);
        measure_pattern(&field.pattern, depth, counts);
    }
}

#[allow(clippy::expect_used)]
fn add_string(counts: &mut super::ExpressionCounts, value: &str) {
    counts.string_bytes = counts
        .string_bytes
        .checked_add(u64::try_from(value.len()).expect("host string bytes fit u64"))
        .expect("materialized pattern strings fit u64");
}
