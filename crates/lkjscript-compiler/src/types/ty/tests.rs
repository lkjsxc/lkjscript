use super::{parse_one, Type};

#[test]
fn product_types_are_explicit_and_nominal_by_name() {
    let atoms = vec!["list".into(), "product".into(), "point".into()];
    assert_eq!(
        parse_one(&atoms, 0),
        Ok((Type::List(Box::new(Type::Product("point".into()))), 3))
    );
    assert!(parse_one(&["product".into()], 0).is_err());
    assert!(parse_one(&["product".into(), "Point".into()], 0).is_err());
}

#[test]
fn only_canonical_numeric_type_names_are_accepted() {
    for (name, expected) in [("i64", Type::I64), ("f64", Type::F64)] {
        assert_eq!(parse_one(&[name.into()], 0).ok(), Some((expected, 1)));
    }
    for name in [
        "I32", "U32", "U64", "F32", "I128", "U8", "F16", "I64", "F64", "i32", "u32", "u64", "f32",
        "i128", "Int", "Float",
    ] {
        assert!(parse_one(&[name.into()], 0).is_err(), "accepted {name}");
    }
}
