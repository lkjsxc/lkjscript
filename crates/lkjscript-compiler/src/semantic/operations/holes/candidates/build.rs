use crate::hir::Type;
use crate::semantic::schema::*;

use super::super::site::HoleSite;

pub(super) fn literal_expressions(
    tree: &crate::source::ValidatedSourceTree,
    ty: &Type,
) -> Vec<(CandidateCategory, Expression)> {
    let mut result: Vec<(CandidateCategory, Expression)> = match ty {
        Type::Unit => vec![Expression::Unit {}],
        Type::Bool => vec![
            Expression::Bool { value: false },
            Expression::Bool { value: true },
        ],
        Type::I64 => vec![
            Expression::I64 { value: 0 },
            Expression::I64 { value: 1 },
            Expression::I64 { value: -1 },
        ],
        Type::F64 => vec![
            Expression::F64 {
                value: "0.0".into(),
            },
            Expression::F64 {
                value: "1.0".into(),
            },
        ],
        Type::Str => vec![
            Expression::String {
                value: String::new(),
            },
            Expression::String {
                value: "value".into(),
            },
        ],
        _ => Vec::new(),
    }
    .into_iter()
    .map(|expression| (CandidateCategory::ExactLiteral, expression))
    .collect();
    if let Some(expression) = super::super::validate::witness(tree, ty, 0) {
        let category = match ty {
            Type::Product(_) => CandidateCategory::ProductConstructor,
            Type::Enum { id, .. } if id.bytes() == lkjscript_core::OPTION_ID => {
                CandidateCategory::OptionConstructor
            }
            Type::Enum { id, .. } if id.bytes() == lkjscript_core::RESULT_ID => {
                CandidateCategory::ResultConstructor
            }
            _ => CandidateCategory::ExactLiteral,
        };
        if !matches!(
            ty,
            Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str
        ) {
            result.push((category, expression));
        }
    }
    result
}

pub(super) fn binding_expression(entity: &ScopeEntity, expected: &Type) -> Option<Expression> {
    let actual = super::super::scope::parse_canonical(&entity.instantiated_type)?;
    if actual == *expected {
        return Some(if matches!(actual, Type::Owned(_)) {
            Expression::Move {
                name: entity.name.clone(),
            }
        } else {
            Expression::NameReference {
                name: entity.name.clone(),
            }
        });
    }
    match (&actual, expected) {
        (Type::Owned(actual), Type::Ref(expected)) if actual == expected => {
            Some(Expression::Borrow {
                name: entity.name.clone(),
            })
        }
        (Type::Owned(actual), Type::RefMut(expected)) if actual == expected => {
            Some(Expression::BorrowMut {
                name: entity.name.clone(),
            })
        }
        _ => None,
    }
}

pub(super) fn candidate(
    site: &HoleSite<'_>,
    ty: &Type,
    category: CandidateCategory,
    expression: Expression,
) -> Option<HoleCandidate> {
    let canonical_source = source(&expression, site.source.span)?;
    let identity = crate::semantic::tree::hex(&lkjscript_core::sha256(
        format!("{:?}\0{canonical_source}", category).as_bytes(),
    ));
    let cost = node_cost(&expression);
    let effects = super::super::candidate_support::effects(site, &expression);
    let rank = CandidateRank {
        category: category as u16,
        effect_cost: effects.len() as u16,
        ownership_cost: u16::from(!matches!(
            super::super::types::ownership(ty),
            OwnershipAccess::Copy,
        )),
        construction_cost: cost,
        canonical_source: canonical_source.clone(),
        identity: identity.clone(),
    };
    let edit = ExactSemanticEdit::ReplaceHole {
        declaration_key: site.declaration_key.clone(),
        node: site.node,
        node_fingerprint: crate::semantic::tree::fingerprint(site.source),
        expression: expression.clone(),
    };
    Some(HoleCandidate {
        identity,
        category,
        rank,
        result_type: if matches!(
            category,
            CandidateCategory::ControlForm | CandidateCategory::NeverForm
        ) && matches!(
            &expression,
            Expression::Return { .. }
                | Expression::Break { .. }
                | Expression::Continue {}
                | Expression::Trap { .. }
                | Expression::Exit { .. }
        ) {
            "Never".into()
        } else {
            super::super::types::canonical(ty)
        },
        effects,
        ownership: if matches!(
            &expression,
            Expression::Return { .. }
                | Expression::Break { .. }
                | Expression::Continue {}
                | Expression::Trap { .. }
                | Expression::Exit { .. }
        ) {
            OwnershipAccess::Unavailable
        } else {
            super::super::types::ownership(ty)
        },
        capabilities: super::capabilities::required(site.tree, &expression),
        construction_cost: cost,
        expression,
        snippets: vec![ConcreteSnippet {
            source: canonical_source,
            complete: true,
        }],
        edits: vec![edit],
        inclusion_reason: if matches!(
            category,
            CandidateCategory::DirectFunction
                | CandidateCategory::DirectBuiltin
                | CandidateCategory::ExactConversion
        ) {
            InclusionReason::CheckerValidatedCall
        } else {
            InclusionReason::ExactTypeAndConstraints
        },
        validating_checker: "lkjscript HIR type/effect and ownership checker".into(),
    })
}

fn source(expression: &Expression, span: crate::source::SourceSpan) -> Option<String> {
    expression
        .to_source(span)
        .ok()
        .map(|node| crate::source::format_node_source(&node))
}

fn node_cost(expression: &Expression) -> u32 {
    let mut counts = ExpressionCounts::default();
    expression.measure(1, &mut counts);
    u32::try_from(counts.nodes).unwrap_or(u32::MAX)
}
