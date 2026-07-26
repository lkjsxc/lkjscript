use super::usefulness::Constructor;
use crate::analyze::*;

pub(super) fn constructor(pattern: &MatchPattern) -> Option<Constructor> {
    match pattern {
        MatchPattern::Bool(value) => Some(Constructor::Bool(*value)),
        MatchPattern::I64(value) => Some(Constructor::I64(*value)),
        MatchPattern::Variant { variant, .. } => Some(Constructor::Variant(*variant)),
        MatchPattern::Product { product, .. } => Some(Constructor::Product(*product)),
        MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. } => None,
    }
}

pub(super) fn constructors(matrix: &[Vec<MatchPattern>]) -> Vec<Constructor> {
    let mut result = Vec::new();
    for item in matrix
        .iter()
        .filter_map(|row| row.first())
        .filter_map(constructor)
    {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

pub(super) fn default_matrix(matrix: &[Vec<MatchPattern>]) -> Vec<Vec<MatchPattern>> {
    matrix
        .iter()
        .filter_map(|row| match row.first() {
            Some(MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. }) => {
                Some(row[1..].to_vec())
            }
            _ => None,
        })
        .collect()
}

pub(super) fn specialize_matrix(
    matrix: &[Vec<MatchPattern>],
    constructor: &Constructor,
    fields: &[Type],
) -> Vec<Vec<MatchPattern>> {
    matrix
        .iter()
        .filter_map(|row| {
            let mut head = specialize_pattern(row.first()?, constructor, fields)?;
            head.extend_from_slice(&row[1..]);
            Some(head)
        })
        .collect()
}

pub(super) fn specialize_pattern(
    pattern: &MatchPattern,
    target: &Constructor,
    fields: &[Type],
) -> Option<Vec<MatchPattern>> {
    match pattern {
        MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. } => Some(
            fields
                .iter()
                .cloned()
                .map(|ty| MatchPattern::Wildcard { ty })
                .collect(),
        ),
        MatchPattern::Variant {
            variant, fields, ..
        } if &Constructor::Variant(*variant) == target => {
            Some(fields.iter().map(|field| field.pattern.clone()).collect())
        }
        MatchPattern::Product {
            product, fields, ..
        } if &Constructor::Product(*product) == target => {
            Some(fields.iter().map(|field| field.pattern.clone()).collect())
        }
        other if constructor(other).as_ref() == Some(target) => Some(Vec::new()),
        _ => None,
    }
}
