use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LayoutShape {
    Unit,
    Bool,
    I64,
    F64,
    RegionProduct(u32, [u8; 32]),
    List(u64, crate::LayoutIdentity, u64),
}

pub(super) fn verify_layout_identities(
    declarations: &[FunctionDeclaration],
) -> Result<(), VerificationError> {
    let mut identities = HashMap::new();
    for (identity, shape) in [
        (ValueType::Unit.layout_identity(), LayoutShape::Unit),
        (ValueType::Bool.layout_identity(), LayoutShape::Bool),
        (ValueType::I64.layout_identity(), LayoutShape::I64),
        (ValueType::F64.layout_identity(), LayoutShape::F64),
    ] {
        identities.insert(identity, shape);
    }
    for declaration in declarations {
        for value_type in declaration
            .signature
            .parameters()
            .iter()
            .copied()
            .chain(std::iter::once(declaration.signature.result()))
            .chain(
                declaration
                    .body
                    .iter()
                    .flat_map(|function| function.locals.iter().map(|local| local.value_type)),
            )
            .chain(
                declaration
                    .body
                    .iter()
                    .flat_map(|function| function.values.iter().map(|value| value.value_type)),
            )
        {
            let Some(reference_type) = value_type.reference_type() else {
                continue;
            };
            let (identity, shape) = match reference_type {
                ReferenceType::RegionProduct(identity, digest) => {
                    let Some(product) = identity.get().checked_sub(32) else {
                        return Err(VerificationError::TypeMismatch(
                            "structural layout identity",
                        ));
                    };
                    if u16::try_from(product).is_err() {
                        return Err(VerificationError::TypeMismatch(
                            "structural layout identity",
                        ));
                    }
                    (identity, LayoutShape::RegionProduct(product, digest))
                }
                ReferenceType::List(identity, semantic, element, element_semantic) => {
                    if semantic == 0 || element_semantic == 0 {
                        return Err(VerificationError::TypeMismatch(
                            "structural semantic identity",
                        ));
                    }
                    (
                        identity,
                        LayoutShape::List(semantic, element, element_semantic),
                    )
                }
            };
            if identities
                .insert(identity, shape)
                .is_some_and(|old| old != shape)
            {
                return Err(VerificationError::TypeMismatch(
                    "structural layout identity",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn verify_signature(
    function: FunctionId,
    signature: &Signature,
) -> Result<(), VerificationError> {
    if signature.machine_parameter_count() > 2 {
        return Err(VerificationError::UnsupportedSignature(function));
    }
    for value_type in signature
        .parameters()
        .iter()
        .copied()
        .chain(std::iter::once(signature.result()))
    {
        let valid = match value_type {
            ValueType::StaticString(value_type) | ValueType::StructuralOwner(value_type) => {
                value_type.is_valid()
            }
            ValueType::StructuralView(view) => view.is_valid(),
            ValueType::StructuralDestination(destination) => destination.is_valid(),
            _ => true,
        };
        if !valid {
            return Err(VerificationError::UnsupportedSignature(function));
        }
    }
    Ok(())
}
