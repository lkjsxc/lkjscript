use super::*;

#[test]
fn sha256_has_an_exact_signature_and_memory_effects() {
    assert_eq!(Operation::from_name("sha256"), Some(Operation::SysSha256));
    assert_eq!(
        Operation::SysSha256.resolve_types(&[Type::ByteSlice]),
        Ok((function(vec![Type::ByteSlice], Type::Bytes), Type::Bytes))
    );
    assert!(Operation::SysSha256.resolve_types(&[Type::Bytes]).is_err());
    assert_eq!(
        Operation::SysSha256.effects(),
        EffectSet::ALLOCATES
            .union(EffectSet::READS_MEMORY)
            .union(EffectSet::MAY_TRAP)
    );
}
