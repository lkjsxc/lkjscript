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
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Bytes | Type::ByteVector | Type::ByteSlice | Type::ByteSliceMut => return true,
            Type::List(inner) => pending.push(inner),
            Type::Enum { arguments, .. } => pending.extend(arguments),
            Type::Fn { params, ret } => {
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => pending.push(body),
            _ => {}
        }
    }
    false
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
    matches!(ty, Type::Bytes | Type::ByteVector)
}

pub(in crate::ownership) fn is_affine_resource(ty: &Type) -> bool {
    matches!(ty, Type::Resource(_))
}

pub(in crate::ownership) fn is_ref(ty: &Type) -> bool {
    matches!(ty, Type::ByteSlice)
}

pub(in crate::ownership) fn is_ref_mut(ty: &Type) -> bool {
    matches!(ty, Type::ByteSliceMut)
}
