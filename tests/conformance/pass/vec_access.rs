struct Affine {
    value: Box<i64>,
}
pub fn probe() -> i64 {
    let empty: Vec<Affine> = Vec::new();
    assert!(empty.is_empty());
    assert!(empty.first().is_none());
    assert!(empty.get(0usize).is_none());
    let mut values: Vec<Affine> = Vec::new();
    values.push(Affine {
        value: Box::new(20i64),
    });
    values.push(Affine {
        value: Box::new(22i64),
    });
    assert!(!values.is_empty());
    assert!(values.get(2usize).is_none());
    assert!(values.get(18446744073709551615usize).is_none());
    let first = values.first().unwrap();
    let present = values.get(1usize);
    let last = present.unwrap();
    drop(present);
    let total = *first.value + *last.value;
    let shared = &values;
    let nested = &shared;
    let nested_first = nested.first().unwrap();
    assert!(*nested_first.value == 20i64);
    total
}
