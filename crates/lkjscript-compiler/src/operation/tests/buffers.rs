use super::*;

#[test]
fn lossless_bulk_byte_operations_have_exact_signatures_and_effects() {
    let result_i64 = Type::Result(Box::new(Type::I64), Box::new(Type::Str));
    let result_str = Type::Result(Box::new(Type::Str), Box::new(Type::Str));
    assert_eq!(
        Operation::from_name("buf-from-str"),
        Some(Operation::BufFromStr)
    );
    assert_eq!(
        Operation::from_name("buf-to-str"),
        Some(Operation::BufToStr)
    );
    assert_eq!(
        Operation::SysReadInto.resolve_types(&[Type::Handle, Type::Buf, Type::I64, Type::I64]),
        Ok((
            function(
                vec![Type::Handle, Type::Buf, Type::I64, Type::I64],
                result_i64.clone()
            ),
            result_i64.clone(),
        ))
    );
    assert_eq!(
        Operation::BufToStr.resolve_types(&[Type::Buf]),
        Ok((function(vec![Type::Buf], result_str.clone()), result_str))
    );
    assert_eq!(
        Operation::BufFromStr.effects(),
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
