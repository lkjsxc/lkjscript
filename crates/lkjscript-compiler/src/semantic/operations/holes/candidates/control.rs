use crate::hir::Type;
use crate::semantic::schema::{CandidateCategory, Expression};

use super::super::site::HoleSite;

pub(super) fn expressions(
    site: &HoleSite<'_>,
    expected: &Type,
) -> Vec<(CandidateCategory, Expression)> {
    let mut result = Vec::new();
    if let (Some(result_type), Some(value)) = (
        super::super::validate::type_expression(expected),
        super::super::validate::witness(site.tree, expected, 0),
    ) {
        result.push((
            CandidateCategory::ControlForm,
            Expression::Loop {
                result_type,
                body: vec![Expression::Break {
                    value: Box::new(value),
                }],
            },
        ));
    }
    if let Some(value) = super::super::validate::witness(site.tree, &site.return_type, 0) {
        result.push((
            CandidateCategory::ControlForm,
            Expression::Return {
                value: Box::new(value),
            },
        ));
    }
    if let Some(loop_type) = nearest_loop_type(site) {
        if let Some(value) = super::super::validate::witness(site.tree, &loop_type, 0) {
            result.push((
                CandidateCategory::ControlForm,
                Expression::Break {
                    value: Box::new(value),
                },
            ));
        }
        result.push((CandidateCategory::ControlForm, Expression::Continue {}));
    }
    result.extend([
        (
            CandidateCategory::NeverForm,
            Expression::Trap {
                value: Box::new(Expression::String {
                    value: "typed hole trap".into(),
                }),
            },
        ),
        (
            CandidateCategory::NeverForm,
            Expression::Exit {
                code: Box::new(Expression::I64 { value: 1 }),
            },
        ),
    ]);
    result
}

fn nearest_loop_type(site: &HoleSite<'_>) -> Option<Type> {
    let mut node = site.root;
    let mut result = None;
    for index in &site.path {
        if let crate::source::SyntaxKind::Call { name } = &node.kind {
            if name == "while" {
                result = Some(Type::Unit);
            } else if name == "loop" {
                result = node
                    .children
                    .first()
                    .and_then(super::super::types::type_form);
            }
        }
        node = node.children.get(*index)?;
    }
    result
}
