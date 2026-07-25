use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LayoutShape {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Buf,
    Product(u32),
    List(crate::LayoutIdentity),
    Option(crate::LayoutIdentity),
    Result(crate::LayoutIdentity, crate::LayoutIdentity),
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
        (
            ValueType::Reference(ReferenceType::Str).layout_identity(),
            LayoutShape::Str,
        ),
        (
            ValueType::Reference(ReferenceType::Buf).layout_identity(),
            LayoutShape::Buf,
        ),
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
                ReferenceType::Buf | ReferenceType::Str => continue,
                ReferenceType::Product(identity) => {
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
                    (identity, LayoutShape::Product(product))
                }
                ReferenceType::List(identity, element) => (identity, LayoutShape::List(element)),
                ReferenceType::Option(identity, payload) => {
                    (identity, LayoutShape::Option(payload))
                }
                ReferenceType::Result(identity, ok, error) => {
                    (identity, LayoutShape::Result(ok, error))
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
    Ok(())
}
