use super::*;

#[test]
fn durable_file_operations_have_exact_signatures_and_effects() {
    let result_handle = crate::types::result_type(Type::Handle, crate::types::system_error_type());
    let result_unit = crate::types::result_type(Type::Unit, crate::types::system_error_type());
    assert_eq!(
        Operation::from_name("sys-open-append"),
        Some(Operation::SysOpenAppend)
    );
    assert_eq!(
        Operation::from_name("sys-random-fill"),
        Some(Operation::SysRandomFill)
    );
    assert_eq!(
        Operation::SysOpenCreateNew.resolve_types(&[
            Type::Capability(lkjscript_core::CapabilityKind::FileSystem),
            Type::Path,
        ]),
        Ok((
            function(
                vec![
                    Type::Capability(lkjscript_core::CapabilityKind::FileSystem),
                    Type::Path,
                ],
                result_handle.clone(),
            ),
            result_handle
        ))
    );
    assert!(Operation::SysOpenCreateNew
        .resolve_types(&[
            Type::Capability(lkjscript_core::CapabilityKind::FileSystem),
            Type::Str,
        ])
        .is_err());
    assert_eq!(
        Operation::SysTruncate.resolve_types(&[Type::Handle, Type::I64]),
        Ok((
            function(vec![Type::Handle, Type::I64], result_unit.clone()),
            result_unit.clone(),
        ))
    );
    assert_eq!(
        Operation::SysRandomFill.resolve_types(&[
            Type::Capability(lkjscript_core::CapabilityKind::Entropy),
            Type::Buf,
            Type::I64,
            Type::I64,
        ]),
        Ok((
            function(
                vec![
                    Type::Capability(lkjscript_core::CapabilityKind::Entropy),
                    Type::Buf,
                    Type::I64,
                    Type::I64,
                ],
                result_unit,
            ),
            crate::types::result_type(Type::Unit, crate::types::system_error_type()),
        ))
    );
    assert_eq!(
        Operation::SysFsync.effects(),
        EffectSet::HOST_IO
            .union(EffectSet::ALLOCATES)
            .union(EffectSet::MAY_TRAP)
    );
}
