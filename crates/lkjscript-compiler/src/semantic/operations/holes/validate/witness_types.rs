use crate::hir::Type;
use crate::semantic::schema::TypeExpression;

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
        Type::Bytes => T::Bytes {},
        Type::ByteVector => T::ByteVector {},
        Type::ByteSlice => T::ByteSlice {},
        Type::ByteSliceMut => T::ByteSliceMut {},
        Type::Path => T::Path {},
        Type::Capability(kind) => T::Capability {
            capability: kind.as_str().into(),
        },
        Type::Symbol => T::Symbol {},
        Type::Resource(kind) => T::Resource {
            resource: kind.as_str().into(),
        },
        Type::Product(name) => T::Product { name: name.clone() },
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::OPTION_ID && arguments.len() == 1 =>
        {
            T::Option {
                value: Box::new(type_expression(&arguments[0])?),
            }
        }
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::RESULT_ID && arguments.len() == 2 =>
        {
            T::Result {
                ok: Box::new(type_expression(&arguments[0])?),
                error: Box::new(type_expression(&arguments[1])?),
            }
        }
        Type::Enum {
            id,
            name,
            arguments,
        } if matches!(
            id.bytes(),
            lkjscript_core::NUMERIC_ERROR_ID
                | lkjscript_core::UTF8_ERROR_ID
                | lkjscript_core::SYSTEM_ERROR_ID
        ) =>
        {
            T::Enum {
                name: name.clone(),
                arguments: arguments
                    .iter()
                    .map(type_expression)
                    .collect::<Option<Vec<_>>>()?,
            }
        }
        Type::Enum { .. } => return None,
        Type::Param(name) => T::Variable { name: name.clone() },
        Type::List(inner) => T::List {
            element: Box::new(type_expression(inner)?),
        },
        Type::Fn { .. } | Type::Forall { .. } => return None,
    })
}
