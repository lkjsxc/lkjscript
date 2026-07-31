use super::*;

#[test]
fn byte_data_replacements_have_exact_signatures_and_effects() {
    let result_i64 = crate::types::result_type(Type::I64, crate::types::system_error_type());
    let result_str = crate::types::result_type(Type::Str, crate::types::utf8_error_type());
    assert_eq!(
        Operation::from_name("convert-string-to-bytes"),
        Some(Operation::ConvertStringToBytes)
    );
    assert_eq!(
        Operation::from_name("convert-bytes-to-string"),
        Some(Operation::ConvertBytesToString)
    );
    assert_eq!(
        Operation::SysReadInto.resolve_types(&[
            Type::Resource(lkjscript_core::ResourceKind::FileReader),
            Type::ByteSliceMut,
        ]),
        Ok((
            function(
                vec![
                    Type::Resource(lkjscript_core::ResourceKind::FileReader),
                    Type::ByteSliceMut,
                ],
                result_i64.clone()
            ),
            result_i64,
        ))
    );
    assert_eq!(
        Operation::ConvertBytesToString.resolve_types(&[Type::Bytes]),
        Ok((function(vec![Type::Bytes], result_str.clone()), result_str))
    );
    assert!(Operation::from_name("buf-new").is_none());
    assert!(Operation::from_name("convert-string-to-buf").is_none());
    assert_eq!(
        Operation::ConvertStringToBytes.effects(),
        EffectSet::ALLOCATES
            .union(EffectSet::READS_MEMORY)
            .union(EffectSet::MAY_TRAP)
    );
    assert_eq!(
        Operation::SysReadInto.effects(),
        EffectSet::HOST_IO
            .union(EffectSet::ALLOCATES)
            .union(EffectSet::WRITES_MEMORY)
            .union(EffectSet::MAY_TRAP)
    );
}
