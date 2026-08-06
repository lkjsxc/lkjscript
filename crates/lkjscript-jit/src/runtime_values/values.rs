use crate::*;

pub(crate) fn reference_layout_key(reference_type: ReferenceType) -> u64 {
    let mut bytes = b"lkjscript.native-reference-layout\0".to_vec();
    match reference_type {
        ReferenceType::List(layout, list_semantic, element, element_semantic) => {
            bytes.push(0);
            append_layout_identity(&mut bytes, layout);
            bytes.extend_from_slice(&list_semantic.to_be_bytes());
            append_layout_identity(&mut bytes, element);
            bytes.extend_from_slice(&element_semantic.to_be_bytes());
        }
        ReferenceType::RegionProduct(layout, identity) => {
            bytes.push(1);
            append_layout_identity(&mut bytes, layout);
            bytes.extend_from_slice(&identity);
        }
    }
    let digest = lkjscript_core::sha256(&bytes);
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}

fn append_layout_identity(bytes: &mut Vec<u8>, identity: lkjscript_native::LayoutIdentity) {
    use lkjscript_native::LayoutIdentity;

    match identity {
        LayoutIdentity::Unit => bytes.push(0),
        LayoutIdentity::Bool => bytes.push(1),
        LayoutIdentity::I64 => bytes.push(2),
        LayoutIdentity::F64 => bytes.push(3),
        LayoutIdentity::StructuralKey => bytes.push(4),
        LayoutIdentity::StaticBytes => bytes.push(5),
        LayoutIdentity::MemoryWitnessLocator => bytes.push(6),
        LayoutIdentity::Structural(value) => {
            bytes.push(7);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        LayoutIdentity::Capability(kind) => {
            bytes.push(8);
            bytes.push(kind as u8);
        }
        LayoutIdentity::Resource(kind) => {
            bytes.push(9);
            bytes.push(kind as u8);
        }
        LayoutIdentity::LoanBytes => bytes.push(10),
        LayoutIdentity::UniqueByteVector => bytes.push(11),
        LayoutIdentity::LoanByteSlice => bytes.push(12),
        LayoutIdentity::LoanByteSliceMut => bytes.push(13),
        LayoutIdentity::UniqueBytes => bytes.push(14),
        LayoutIdentity::Product(value) => {
            bytes.push(15);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
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

#[cfg(test)]
mod tests {
    use super::reference_layout_key;
    use lkjscript_native::{LayoutIdentity, ReferenceType, ValueType};

    #[test]
    fn high_product_layouts_do_not_alias_opaque_or_low_product_domains() {
        let high = u64::from(u32::MAX) + 1;
        let product = ReferenceType::RegionProduct(LayoutIdentity::product(high), [1; 32]);
        let low = ReferenceType::RegionProduct(LayoutIdentity::product(0), [1; 32]);
        let opaque = ReferenceType::RegionProduct(LayoutIdentity::new(high), [1; 32]);
        assert_ne!(reference_layout_key(product), reference_layout_key(low));
        assert_ne!(reference_layout_key(product), reference_layout_key(opaque));
        assert_ne!(LayoutIdentity::new(1), ValueType::Unit.layout_identity());
    }
}
