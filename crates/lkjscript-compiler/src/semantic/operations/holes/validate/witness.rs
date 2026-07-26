use crate::hir::Type;
use crate::semantic::schema::{Expression, ExpressionField, TypeExpression};
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
            if matches!(actual, Type::Owned(_)) {
                Some(Expression::Move { name: name.into() })
            } else {
                Some(Expression::NameReference { name: name.into() })
            }
        } else {
            match (&actual, expected) {
                (Type::Owned(actual), Type::Ref(expected)) if actual == expected => {
                    Some(Expression::Borrow { name: name.into() })
                }
                (Type::Owned(actual), Type::RefMut(expected)) if actual == expected => {
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

pub(crate) fn witness(tree: &ValidatedSourceTree, ty: &Type, depth: u32) -> Option<Expression> {
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
        Type::Option(inner) => Expression::None {
            value_type: type_expression(inner)?,
        },
        Type::Product(name) => product_witness(tree, name, depth + 1)?,
        Type::Result(ok, error) if is_numeric_error(error) => Expression::BuiltinCall {
            operation: crate::semantic::schema::ClosedBuiltinOperation(match ok.as_ref() {
                Type::F64 => crate::hir::Operation::F64FromI64Exact,
                Type::I64 => crate::hir::Operation::I64FromF64Exact,
                _ => return None,
            }),
            arguments: vec![match ok.as_ref() {
                Type::F64 => Expression::I64 { value: 0 },
                Type::I64 => Expression::F64 {
                    value: "0.0".into(),
                },
                _ => return None,
            }],
        },
        _ => return None,
    })
}

fn product_witness(tree: &ValidatedSourceTree, name: &str, depth: u32) -> Option<Expression> {
    let nodes = crate::semantic::tree::source_nodes(tree);
    let declaration = tree.declarations().iter().find(|item| {
        item.kind() == crate::source::DeclarationKind::Product && item.name() == name
    })?;
    let root = nodes.get(declaration.node().index() as usize)?;
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

pub(in crate::semantic::operations::holes) fn type_expression(ty: &Type) -> Option<TypeExpression> {
    use TypeExpression as T;
    Some(match ty {
        Type::Never => T::Never {},
        Type::Unit => T::Unit {},
        Type::Bool => T::Bool {},
        Type::I64 => T::I64 {},
        Type::F64 => T::F64 {},
        Type::Str => T::String {},
        Type::Buf => T::Buffer {},
        Type::Symbol => T::Symbol {},
        Type::Handle => T::Handle {},
        Type::Product(name) => T::Product { name: name.clone() },
        Type::Enum {
            id,
            name,
            arguments,
        } if id.bytes() == lkjscript_core::NUMERIC_ERROR_ID => T::Enum {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(type_expression)
                .collect::<Option<Vec<_>>>()?,
        },
        Type::Enum { .. } => return None,
        Type::Param(name) => T::Variable { name: name.clone() },
        Type::Owned(inner) => T::Owned {
            inner: Box::new(type_expression(inner)?),
        },
        Type::Ref(inner) => T::Ref {
            inner: Box::new(type_expression(inner)?),
        },
        Type::RefMut(inner) => T::RefMut {
            inner: Box::new(type_expression(inner)?),
        },
        Type::List(inner) => T::List {
            element: Box::new(type_expression(inner)?),
        },
        Type::Option(inner) => T::Option {
            value: Box::new(type_expression(inner)?),
        },
        Type::Result(ok, error) => T::Result {
            ok: Box::new(type_expression(ok)?),
            error: Box::new(type_expression(error)?),
        },
        Type::Fn { .. } | Type::Forall { .. } => return None,
    })
}

fn is_numeric_error(ty: &Type) -> bool {
    matches!(ty, Type::Enum { id, arguments, .. }
        if id.bytes() == lkjscript_core::NUMERIC_ERROR_ID && arguments.is_empty())
}
