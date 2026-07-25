use super::*;

#[test]
fn durable_file_operations_have_exact_signatures_and_effects() {
    let result_handle = Type::Result(Box::new(Type::Handle), Box::new(Type::Str));
    let result_unit = Type::Result(Box::new(Type::Unit), Box::new(Type::Str));
    assert_eq!(
        Operation::from_name("sys-open-append"),
        Some(Operation::SysOpenAppend)
    );
    assert_eq!(
        Operation::from_name("sys-random-fill"),
        Some(Operation::SysRandomFill)
    );
    assert_eq!(
        Operation::SysOpenCreateNew.resolve_types(&[Type::Str]),
        Ok((
            function(vec![Type::Str], result_handle.clone()),
            result_handle
        ))
    );
    assert_eq!(
        Operation::SysTruncate.resolve_types(&[Type::Handle, Type::I64]),
        Ok((
            function(vec![Type::Handle, Type::I64], result_unit.clone()),
            result_unit.clone(),
        ))
    );
    assert_eq!(
        Operation::SysRandomFill.resolve_types(&[Type::Buf, Type::I64, Type::I64]),
        Ok((
            function(vec![Type::Buf, Type::I64, Type::I64], result_unit),
            Type::Result(Box::new(Type::Unit), Box::new(Type::Str)),
        ))
    );
    assert_eq!(
        Operation::SysFsync.effects(),
        EffectSet::HOST_IO
            .union(EffectSet::ALLOCATES)
            .union(EffectSet::MAY_TRAP)
    );
}
