use super::*;

pub(super) fn numeric_conversion_canonical(
    kind: StructuralNumericConversion,
    success: &StructuralAggregateDescriptor,
    failure: &StructuralAggregateDescriptor,
    errors: &[StructuralAggregateDescriptor],
) -> bool {
    let success_kind = match kind {
        StructuralNumericConversion::F64FromI64Exact => StructuralKind::F64,
        StructuralNumericConversion::I64FromF64Exact
        | StructuralNumericConversion::I64FromF64Truncating => StructuralKind::I64,
    };
    success.canonical()
        && failure.canonical()
        && success.value_type() == failure.value_type()
        && matches!(success.kind(), StructuralAggregateKind::Enum(0))
        && matches!(failure.kind(), StructuralAggregateKind::Enum(1))
        && success.fields().len() == 1
        && success.fields()[0].kind() == success_kind
        && failure.fields().len() == 1
        && errors.len() == 4
        && errors.iter().all(|error| {
            error.canonical()
                && error.fields().is_empty()
                && error.value_type() == failure.fields()[0]
        })
        && [0_u16, 1, 2, 3].into_iter().all(|tag| {
            errors.iter().any(|error| {
                matches!(error.kind(), StructuralAggregateKind::Enum(actual) if actual == tag)
            })
        })
}

pub(super) fn payload_matches(
    value_type: StructuralTypeIdentity,
    payload: StructuralPayloadKind,
) -> bool {
    matches!(
        (value_type.kind(), payload),
        (StructuralKind::String, StructuralPayloadKind::String)
            | (StructuralKind::Path, StructuralPayloadKind::Path)
            | (StructuralKind::Bytes, StructuralPayloadKind::Bytes)
            | (
                StructuralKind::ByteVector,
                StructuralPayloadKind::ByteVector
            )
    )
}

pub(super) fn byte_payload(kind: StructuralKind) -> bool {
    matches!(
        kind,
        StructuralKind::String
            | StructuralKind::Path
            | StructuralKind::Bytes
            | StructuralKind::ByteVector
    )
}
