use super::*;

#[test]
fn durable_file_operations_have_exact_signatures_and_effects() {
    let result_resource = crate::types::result_type(
        Type::Resource(lkjscript_core::ResourceKind::FileWriter),
        crate::types::system_error_type(),
    );
    let result_unit = crate::types::result_type(Type::Unit, crate::types::system_error_type());
    assert_eq!(
        Operation::from_name("open-file-appender"),
        Some(Operation::SysOpenAppend)
    );
    assert_eq!(
        Operation::from_name("fill-random"),
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
                result_resource.clone(),
            ),
            result_resource
        ))
    );
    assert!(Operation::SysOpenCreateNew
        .resolve_types(&[
            Type::Capability(lkjscript_core::CapabilityKind::FileSystem),
            Type::Str,
        ])
        .is_err());
    assert_eq!(
        Operation::SysTruncate.resolve_types(&[
            Type::Resource(lkjscript_core::ResourceKind::FileWriter),
            Type::I64,
        ]),
        Ok((
            function(
                vec![
                    Type::Resource(lkjscript_core::ResourceKind::FileWriter),
                    Type::I64,
                ],
                result_unit.clone(),
            ),
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

#[test]
fn generic_resource_operations_enforce_exact_kind_sets() {
    use lkjscript_core::ResourceKind::{
        Directory, FileAppender, FileReader, FileWriter, InputStream, OutputStream, TcpListener,
        TcpStream,
    };
    let cases: &[(Operation, &[lkjscript_core::ResourceKind], &[Type])] = &[
        (
            Operation::DropResource,
            &[
                OutputStream,
                FileReader,
                FileWriter,
                FileAppender,
                Directory,
                TcpListener,
                TcpStream,
                lkjscript_core::ResourceKind::SqliteConnection,
                lkjscript_core::ResourceKind::SqliteStatement,
                lkjscript_core::ResourceKind::TerminalSession,
            ],
            &[],
        ),
        (
            Operation::SysReadByte,
            &[InputStream, FileReader, TcpStream],
            &[],
        ),
        (
            Operation::SysWriteByte,
            &[OutputStream, FileWriter, FileAppender, TcpStream],
            &[Type::I64],
        ),
        (
            Operation::SysReadInto,
            &[InputStream, FileReader, TcpStream],
            &[Type::Buf, Type::I64, Type::I64],
        ),
        (
            Operation::SysWriteFrom,
            &[OutputStream, FileWriter, FileAppender, TcpStream],
            &[Type::Buf, Type::I64, Type::I64],
        ),
        (
            Operation::SysFsync,
            &[FileWriter, FileAppender, Directory],
            &[],
        ),
        (
            Operation::SysTruncate,
            &[FileWriter, FileAppender],
            &[Type::I64],
        ),
        (
            Operation::SysPoll,
            &[InputStream, FileReader, TcpListener, TcpStream],
            &[Type::I64],
        ),
    ];
    for (operation, allowed, tail) in cases {
        let expected_constraint = format!(
            "resource:one-of({})",
            allowed
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        let vocabulary = lkjscript_contracts::operation_by_id(operation.identity());
        assert_eq!(
            vocabulary.map(|record| record.semantics.generic_constraints),
            Some(&[expected_constraint.as_str()][..]),
            "{operation:?}"
        );
        let mut scalar = vec![Type::I64];
        scalar.extend_from_slice(tail);
        assert!(operation.resolve_types(&scalar).is_err(), "{operation:?}");
        for kind in lkjscript_core::ResourceKind::ALL {
            let mut arguments = vec![Type::Resource(kind)];
            arguments.extend_from_slice(tail);
            assert_eq!(
                operation.resolve_types(&arguments).is_ok(),
                allowed.contains(&kind),
                "{operation:?} {kind:?}"
            );
        }
    }
}
