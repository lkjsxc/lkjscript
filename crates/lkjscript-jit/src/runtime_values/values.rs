use crate::*;

pub(crate) fn reference_layout_key(reference_type: ReferenceType) -> u64 {
    match reference_type {
        ReferenceType::List(layout, list_semantic, element, element_semantic) => {
            let mut bytes = b"lkjscript.segmented-list-type\0native-layout".to_vec();
            bytes.extend_from_slice(&layout.get().to_be_bytes());
            bytes.extend_from_slice(&list_semantic.to_be_bytes());
            bytes.extend_from_slice(&element.get().to_be_bytes());
            bytes.extend_from_slice(&element_semantic.to_be_bytes());
            let digest = lkjscript_core::sha256(&bytes);
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ])
        }
        ReferenceType::RegionProduct(layout, _) => (6_u64 << 56) | u64::from(layout.get()),
    }
}

pub(crate) fn install_error(function: FunctionId, error: InstallError) -> EngineError {
    let code = match error {
        InstallError::LimitExceeded(_) => FailureCode::InstallLimit,
        _ => FailureCode::InstallFailure,
    };
    EngineError::new(code, Some(function), error.to_string())
}

pub(crate) fn invocation_error(function: FunctionId, error: InvocationError) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        error.to_string(),
    )
}
