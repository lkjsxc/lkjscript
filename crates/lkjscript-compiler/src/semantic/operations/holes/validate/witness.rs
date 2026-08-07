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

pub(crate) fn witness(tree: &ValidatedSourceTree, ty: &Type) -> Option<Expression> {
    struct ProductFrame {
        product: String,
        remaining: std::vec::IntoIter<(String, Type)>,
        current_field: String,
        values: Vec<ExpressionField>,
    }

    let nodes = crate::semantic::tree::source_nodes(tree);
    let products: std::collections::BTreeMap<_, _> = tree
        .declarations()
        .iter()
        .filter(|item| item.kind() == crate::source::DeclarationKind::Product)
        .filter_map(|item| {
            let index = usize::try_from(item.node().index()).ok()?;
            Some((item.name(), *nodes.get(index)?))
        })
        .collect();
    let mut active_products = std::collections::BTreeSet::new();
    let mut frames = Vec::new();
    let mut current = ty.clone();
    'next_type: loop {
        let mut completed = match &current {
            Type::Product(name) => {
                if !active_products.insert(name.clone()) {
                    return None;
                }
                let mut fields = product_fields(&products, name)?.into_iter();
                if let Some((field, field_type)) = fields.next() {
                    frames.push(ProductFrame {
                        product: name.clone(),
                        remaining: fields,
                        current_field: field,
                        values: Vec::new(),
                    });
                    current = field_type;
                    continue 'next_type;
                }
                active_products.remove(name);
                Expression::ProductValue {
                    product: name.clone(),
                    fields: Vec::new(),
                }
            }
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
        };

        loop {
            let Some(mut frame) = frames.pop() else {
                return Some(completed);
            };
            frame.values.push(ExpressionField {
                name: frame.current_field,
                value: completed,
            });
            if let Some((field, field_type)) = frame.remaining.next() {
                frame.current_field = field;
                frames.push(frame);
                current = field_type;
                continue 'next_type;
            }
            active_products.remove(&frame.product);
            completed = Expression::ProductValue {
                product: frame.product,
                fields: frame.values,
            };
        }
    }
}

fn product_fields(
    products: &std::collections::BTreeMap<&str, &crate::source::SourceNode>,
    name: &str,
) -> Option<Vec<(String, Type)>> {
    let fields = products
        .get(name)?
        .children
        .iter()
        .find(|child| super::super::types::call_is(child, "fields"))?;
    fields
        .children
        .iter()
        .map(|field| {
            let field_name = field
                .children
                .first()
                .and_then(super::super::types::source_name)?
                .to_string();
            let ty = field
                .children
                .get(1)
                .and_then(super::super::types::type_form)?;
            Some((field_name, ty))
        })
        .collect()
}

fn is_numeric_error(ty: &Type) -> bool {
    matches!(ty, Type::Enum { id, arguments, .. }
        if id.bytes() == lkjscript_core::NUMERIC_ERROR_ID && arguments.is_empty())
}
