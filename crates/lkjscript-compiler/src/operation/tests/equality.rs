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
        Type::Option(Box::new(Type::I64)),
        Type::Result(Box::new(Type::Str), Box::new(Type::I64)),
    ] {
        assert!(Operation::EqualValue
            .resolve_types(&[ty.clone(), ty])
            .is_ok());
    }
    for ty in [
        Type::Buf,
        Type::Handle,
        Type::List(Box::new(Type::I64)),
        Type::Param("T".into()),
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

    for ty in [Type::Buf, Type::Handle] {
        assert!(Operation::SameObject
            .resolve_types(&[ty.clone(), ty])
            .is_ok());
    }
    assert!(Operation::SameObject
        .resolve_types(&[Type::I64, Type::I64])
        .is_err());

    let list = Type::List(Box::new(Type::Option(Box::new(Type::Str))));
    assert!(Operation::ListEqual
        .resolve_types(&[list.clone(), list])
        .is_ok());
    let nested = Type::List(Box::new(Type::List(Box::new(Type::I64))));
    assert!(Operation::ListEqual
        .resolve_types(&[nested.clone(), nested])
        .is_err());
    let buffers = Type::List(Box::new(Type::Buf));
    assert!(Operation::ListEqual
        .resolve_types(&[buffers.clone(), buffers])
        .is_err());

    assert!(Operation::F64BitsEqual
        .resolve_types(&[Type::F64, Type::F64])
        .is_ok());
    assert!(Operation::F64BitsEqual
        .resolve_types(&[Type::I64, Type::I64])
        .is_err());
}
