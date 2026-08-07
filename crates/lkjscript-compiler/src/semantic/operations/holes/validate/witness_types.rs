use crate::hir::Type;
use crate::semantic::schema::TypeExpression;

pub(in crate::semantic::operations::holes) fn type_expression(ty: &Type) -> Option<TypeExpression> {
    enum Work<'a> {
        Visit(&'a Type),
        FinishEnum(&'a str, usize),
        FinishList,
        FinishOption,
        FinishResult,
    }

    use TypeExpression as T;
    let mut work = vec![Work::Visit(ty)];
    let mut completed = Vec::new();
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(ty) => match ty {
                Type::Never => completed.push(T::Never {}),
                Type::Unit => completed.push(T::Unit {}),
                Type::Bool => completed.push(T::Bool {}),
                Type::I64 => completed.push(T::I64 {}),
                Type::F64 => completed.push(T::F64 {}),
                Type::Str => completed.push(T::String {}),
                Type::Bytes => completed.push(T::Bytes {}),
                Type::ByteVector => completed.push(T::ByteVector {}),
                Type::ByteSlice => completed.push(T::ByteSlice {}),
                Type::ByteSliceMut => completed.push(T::ByteSliceMut {}),
                Type::Path => completed.push(T::Path {}),
                Type::Capability(kind) => completed.push(T::Capability {
                    capability: kind.as_str().into(),
                }),
                Type::Symbol => completed.push(T::Symbol {}),
                Type::Resource(kind) => completed.push(T::Resource {
                    resource: kind.as_str().into(),
                }),
                Type::Product(name) => completed.push(T::Product { name: name.clone() }),
                Type::Enum { id, arguments, .. }
                    if id.bytes() == lkjscript_core::OPTION_ID && arguments.len() == 1 =>
                {
                    work.push(Work::FinishOption);
                    work.push(Work::Visit(&arguments[0]));
                }
                Type::Enum { id, arguments, .. }
                    if id.bytes() == lkjscript_core::RESULT_ID && arguments.len() == 2 =>
                {
                    work.push(Work::FinishResult);
                    work.push(Work::Visit(&arguments[1]));
                    work.push(Work::Visit(&arguments[0]));
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
                    work.push(Work::FinishEnum(name, arguments.len()));
                    work.extend(arguments.iter().rev().map(Work::Visit));
                }
                Type::Param(name) => completed.push(T::Variable { name: name.clone() }),
                Type::List(inner) => {
                    work.push(Work::FinishList);
                    work.push(Work::Visit(inner));
                }
                Type::Enum { .. } | Type::Fn { .. } | Type::Forall { .. } => return None,
            },
            Work::FinishEnum(name, count) => {
                let split = completed.len().checked_sub(count)?;
                let arguments = completed.split_off(split);
                completed.push(T::Enum {
                    name: name.to_string(),
                    arguments,
                });
            }
            Work::FinishList => {
                let element = completed.pop()?;
                completed.push(T::List {
                    element: Box::new(element),
                });
            }
            Work::FinishOption => {
                let value = completed.pop()?;
                completed.push(T::Option {
                    value: Box::new(value),
                });
            }
            Work::FinishResult => {
                let error = completed.pop()?;
                let ok = completed.pop()?;
                completed.push(T::Result {
                    ok: Box::new(ok),
                    error: Box::new(error),
                });
            }
        }
    }
    if completed.len() == 1 {
        completed.pop()
    } else {
        None
    }
}
