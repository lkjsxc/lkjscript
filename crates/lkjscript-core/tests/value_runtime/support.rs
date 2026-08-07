use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, SemanticTypeIdentity, SemanticValue, StructuralKind, StructuralPublishFailure,
    StructuralType, StructuralValueError, StructuralValueKey, StructuralValueRuntime,
};

pub fn value_type(
    layout: u64,
    semantic: u64,
    kind: StructuralKind,
) -> Result<StructuralType, StructuralValueError> {
    let layout = NonZeroU64::new(layout).ok_or(StructuralValueError::InvariantViolation)?;
    let semantic = NonZeroU64::new(semantic).ok_or(StructuralValueError::InvariantViolation)?;
    Ok(StructuralType::new(
        LayoutIdentity::new(layout),
        SemanticTypeIdentity::new(semantic),
        kind,
    ))
}

pub fn runtime() -> Result<StructuralValueRuntime, StructuralValueError> {
    StructuralValueRuntime::new()
}

pub fn publish(
    runtime: &mut StructuralValueRuntime,
    value: SemanticValue,
) -> Result<StructuralValueKey, StructuralValueError> {
    runtime
        .publish_owned(value)
        .map_err(|failure| failure.error)
}

pub fn publish_failure(
    result: Result<StructuralValueKey, StructuralPublishFailure>,
) -> Result<StructuralPublishFailure, StructuralValueError> {
    match result {
        Ok(_) => Err(StructuralValueError::InvariantViolation),
        Err(failure) => Ok(failure),
    }
}
