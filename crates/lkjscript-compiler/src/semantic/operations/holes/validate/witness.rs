use super::witness_types::type_expression;
use crate::hir::Type;
use crate::semantic::schema::{Expression, ExpressionField};
use crate::source::ValidatedSourceTree;

pub(super) fn scoped_witness(
    site: &super::super::site::HoleSite<'_>,
    expected: &Type,
) -> Option<Expression> {
    let function = if super::super::types::call_is(site.root, "def") {
        site.root
            .children
            .iter()
            .find(|child| super::super::types::call_is(child, "fn"))?
    } else {
        return None;
    };
    let params = function
        .children
        .iter()
        .find(|child| super::super::types::call_is(child, "params"))?;
    let mut index = 0;
    while index < params.children.len() {
        let name = params
            .children
            .get(index)
            .and_then(super::super::types::source_name)?;
        let (actual, used) = super::super::types::parse_type_nodes(&params.children[index + 1..])?;
        let expression = if actual == *expected {
            if matches!(actual, Type::ByteVector) {
                Some(Expression::Move { name: name.into() })
            } else {
                Some(Expression::NameReference { name: name.into() })
            }
        } else {
            match (&actual, expected) {
                (Type::ByteVector, Type::ByteSlice) => {
                    Some(Expression::Borrow { name: name.into() })
                }
                (Type::ByteVector, Type::ByteSliceMut) => {
                    Some(Expression::BorrowMut { name: name.into() })
                }
                _ => None,
            }
        };
        if expression.is_some() {
            return expression;
        }
        index += used + 1;
    }
    None
}

pub(crate) fn witness(tree: &ValidatedSourceTree, ty: &Type, depth: u64) -> Option<Expression> {
    if depth > 8 {
        return None;
    }
    Some(match ty {
        Type::Unit => Expression::Unit {},
        Type::Bool => Expression::Bool { value: false },
        Type::I64 => Expression::I64 { value: 0 },
        Type::F64 => Expression::F64 {
            value: "0.0".into(),
        },
        Type::Str => Expression::String {
            value: String::new(),
        },
        Type::List(inner) => Expression::EmptyList {
            element: type_expression(inner)?,
        },
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::OPTION_ID && arguments.len() == 1 =>
        {
            Expression::None {
                value_type: type_expression(&arguments[0])?,
            }
        }
        Type::Product(name) => product_witness(tree, name, depth + 1)?,
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::RESULT_ID
                && arguments.len() == 2
                && is_numeric_error(&arguments[1]) =>
        {
            let ok = &arguments[0];
            Expression::BuiltinCall {
                operation: crate::semantic::schema::ClosedBuiltinOperation(match ok {
                    Type::F64 => crate::hir::Operation::F64FromI64Exact,
                    Type::I64 => crate::hir::Operation::I64FromF64Exact,
                    _ => return None,
                }),
                arguments: vec![match ok {
                    Type::F64 => Expression::I64 { value: 0 },
                    Type::I64 => Expression::F64 {
                        value: "0.0".into(),
                    },
                    _ => return None,
                }],
            }
        }
        _ => return None,
    })
}

fn product_witness(tree: &ValidatedSourceTree, name: &str, depth: u64) -> Option<Expression> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    let declaration = tree.declarations().iter().find(|item| {
        item.kind() == crate::source::DeclarationKind::Product && item.name() == name
    })?;
    let root = nodes.get(usize::try_from(declaration.node().index()).ok()?)?;
    let fields = root
        .children
        .iter()
        .find(|child| super::super::types::call_is(child, "fields"))?;
    let mut values = Vec::new();
    for field in &fields.children {
        let field_name = field
            .children
            .first()
            .and_then(super::super::types::source_name)?
            .to_string();
        let ty = field
            .children
            .get(1)
            .and_then(super::super::types::type_form)?;
        values.push(ExpressionField {
            name: field_name,
            value: witness(tree, &ty, depth)?,
        });
    }
    Some(Expression::ProductValue {
        product: name.into(),
        fields: values,
    })
}

fn is_numeric_error(ty: &Type) -> bool {
    matches!(ty, Type::Enum { id, arguments, .. }
        if id.bytes() == lkjscript_core::NUMERIC_ERROR_ID && arguments.is_empty())
}
