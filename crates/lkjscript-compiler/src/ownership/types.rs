use crate::ownership::*;

pub(in crate::ownership) fn reject_unsupported_type_placement(ty: &Type) -> Result<()> {
    match ty {
        Type::List(inner) if contains_ownership(inner) => Err(Error::msg(
            "ownership/reference values cannot be stored in List",
        )),
        Type::Enum { arguments, .. } if arguments.iter().any(contains_ownership) => Err(
            Error::msg("ownership/reference values cannot instantiate an enum"),
        ),
        _ => Ok(()),
    }
}

pub(in crate::ownership) fn contains_ownership(ty: &Type) -> bool {
    match ty {
        Type::Owned(_) | Type::Ref(_) | Type::RefMut(_) => true,
        Type::List(inner) => contains_ownership(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(contains_ownership),
        Type::Fn { params, ret } => {
            params.iter().any(contains_ownership) || contains_ownership(ret)
        }
        Type::Forall { body, .. } => contains_ownership(body),
        _ => false,
    }
}

pub(in crate::ownership) fn uses_reference_binding(
    program: &Program,
    expression: &Expr,
) -> Result<bool> {
    for binding in uses(expression) {
        let ty = expression_of_binding(program, binding)?;
        if is_ref(&ty) || is_ref_mut(&ty) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(in crate::ownership) fn expression_of_binding(
    program: &Program,
    binding: BindingId,
) -> Result<Type> {
    program
        .binding(binding)
        .map(|binding| binding.ty.clone())
        .ok_or_else(|| Error::msg("ownership fact references unknown binding"))
}

pub(in crate::ownership) fn is_owned(ty: &Type) -> bool {
    matches!(ty, Type::Owned(inner) if inner.as_ref() == &Type::Buf)
}

pub(in crate::ownership) fn is_ref(ty: &Type) -> bool {
    matches!(ty, Type::Ref(inner) if inner.as_ref() == &Type::Buf)
}

pub(in crate::ownership) fn is_ref_mut(ty: &Type) -> bool {
    matches!(ty, Type::RefMut(inner) if inner.as_ref() == &Type::Buf)
}
