use super::*;

#[test]
fn explicit_equality_families_enforce_static_categories() {
    for ty in [
        Type::Unit,
        Type::Bool,
        Type::I64,
        Type::F64,
        Type::Str,
        Type::Symbol,
        crate::types::option_type(Type::I64),
        crate::types::result_type(Type::Str, Type::I64),
    ] {
        assert!(Operation::EqualValue
            .resolve_types(&[ty.clone(), ty])
            .is_ok());
    }
    for ty in [
        Type::Bytes,
        Type::ByteVector,
        Type::Resource(lkjscript_core::ResourceKind::FileReader),
        Type::List(Box::new(Type::I64)),
        Type::Param("t".into()),
        Type::Fn {
            params: Vec::new(),
            ret: Box::new(Type::Unit),
        },
    ] {
        assert!(Operation::EqualValue
            .resolve_types(&[ty.clone(), ty])
            .is_err());
    }
    assert!(Operation::EqualValue
        .resolve_types(&[Type::I64, Type::F64])
        .is_err());

    let resource = Type::Resource(lkjscript_core::ResourceKind::FileReader);
    assert!(Operation::SameObject
        .resolve_types(&[resource.clone(), resource])
        .is_ok());
    for ty in [Type::I64, Type::Bytes] {
        assert!(Operation::SameObject
            .resolve_types(&[ty.clone(), ty])
            .is_err());
    }

    let list = Type::List(Box::new(crate::types::option_type(Type::Str)));
    assert!(Operation::ListEqual
        .resolve_types(&[list.clone(), list])
        .is_ok());
    let nested = Type::List(Box::new(Type::List(Box::new(Type::I64))));
    assert!(Operation::ListEqual
        .resolve_types(&[nested.clone(), nested])
        .is_ok());
    let owners = Type::List(Box::new(Type::Bytes));
    assert!(Operation::ListEqual
        .resolve_types(&[owners.clone(), owners])
        .is_err());

    assert!(Operation::F64BitsEqual
        .resolve_types(&[Type::F64, Type::F64])
        .is_ok());
    assert!(Operation::F64BitsEqual
        .resolve_types(&[Type::I64, Type::I64])
        .is_err());
}
