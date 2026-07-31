use super::{SemanticPayload, SemanticValue, StructuralFieldPath, StructuralValueError};

pub(super) fn select<'a>(
    mut value: &'a SemanticValue,
    path: &StructuralFieldPath,
) -> Result<&'a SemanticValue, StructuralValueError> {
    for &field in path.as_slice() {
        value = child(value, field)?;
    }
    Ok(value)
}

pub(super) fn select_mut<'a>(
    mut value: &'a mut SemanticValue,
    path: &StructuralFieldPath,
) -> Result<&'a mut SemanticValue, StructuralValueError> {
    for &field in path.as_slice() {
        value = child_mut(value, field)?;
    }
    Ok(value)
}

fn child(value: &SemanticValue, field: u16) -> Result<&SemanticValue, StructuralValueError> {
    let fields = match &value.payload {
        SemanticPayload::Product(fields) => fields,
        SemanticPayload::Enum { active_payload, .. } => active_payload,
        _ => return Err(StructuralValueError::InvalidFieldPath),
    };
    fields
        .get(usize::from(field))
        .ok_or(StructuralValueError::InvalidFieldPath)
}

fn child_mut(
    value: &mut SemanticValue,
    field: u16,
) -> Result<&mut SemanticValue, StructuralValueError> {
    let fields = match &mut value.payload {
        SemanticPayload::Product(fields) => fields,
        SemanticPayload::Enum { active_payload, .. } => active_payload,
        _ => return Err(StructuralValueError::InvalidFieldPath),
    };
    fields
        .get_mut(usize::from(field))
        .ok_or(StructuralValueError::InvalidFieldPath)
}
