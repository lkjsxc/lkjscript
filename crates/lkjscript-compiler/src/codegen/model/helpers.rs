fn unique_value_kind(ty: &SsaType) -> Option<UniqueValueKind> {
    match ty {
        SsaType::Bytes => Some(UniqueValueKind::Bytes),
        SsaType::ByteVector => Some(UniqueValueKind::ByteVector),
        SsaType::ByteSlice => Some(UniqueValueKind::ByteSlice),
        SsaType::ByteSliceMut => Some(UniqueValueKind::ByteSliceMut),
        _ => None,
    }
}

fn resource_return_kind(ty: &SsaType) -> Option<ResourceReturnKind> {
    match ty {
        SsaType::Resource(kind) => Some(ResourceReturnKind::Resource(*kind)),
        SsaType::Enum { id, arguments }
            if id.bytes() == lkjscript_core::RESULT_ID
                && matches!(arguments.as_slice(), [SsaType::Resource(_), _]) =>
        {
            let [SsaType::Resource(kind), _] = arguments.as_slice() else {
                return None;
            };
            Some(ResourceReturnKind::Result(*kind))
        }
        _ => None,
    }
}
