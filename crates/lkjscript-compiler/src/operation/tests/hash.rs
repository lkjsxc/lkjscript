use super::*;

#[test]
fn sha256_has_an_exact_signature_and_memory_effects() {
    let result_buf = crate::types::result_type(Type::Buf, crate::types::system_error_type());
    assert_eq!(
        Operation::from_name("sys-sha256"),
        Some(Operation::SysSha256)
    );
    assert_eq!(
        Operation::SysSha256.resolve_types(&[Type::Buf, Type::I64, Type::I64]),
        Ok((
            function(vec![Type::Buf, Type::I64, Type::I64], result_buf.clone()),
            result_buf,
        ))
    );
    assert!(Operation::SysSha256
        .resolve_types(&[Type::Buf, Type::I64])
        .is_err());
    assert_eq!(
        Operation::SysSha256.effects(),
        EffectSet::ALLOCATES
            .union(EffectSet::READS_MEMORY)
            .union(EffectSet::MAY_TRAP)
    );
}
