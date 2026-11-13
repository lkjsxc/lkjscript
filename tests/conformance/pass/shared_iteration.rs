fn held_items() -> i64 {
    let values = vec![20i64, 22i64];
    let mut iterator = values.iter();
    let first = iterator.next().unwrap();
    let second = iterator.next().unwrap();
    drop(iterator);
    *first + *second
}
fn inferred_pattern() -> i64 {
    let values = vec![(20i64, 1i64), (22i64, 2i64)];
    let mut total = 0;
    for (value, _) in values.iter() {
        total = total + *value;
    }
    total
}
fn ranges() -> i64 {
    let mut total = 0;
    for ref mut value in 0i64..3i64 {
        *value = *value + 10;
        total = total + *value;
    }
    for _ in 0i64..3i64 {
        total = total + 3;
    }
    total
}
fn slice_and_empty() -> i64 {
    let values = [20i64, 22i64];
    let mut total = 0;
    for value in values.as_slice() {
        total = total + *value;
    }
    let empty: Vec<i64> = Vec::new();
    for value in &empty {
        total = total + *value;
    }
    total
}
pub fn probe() -> i64 {
    assert_eq!(held_items(), 42);
    assert_eq!(inferred_pattern(), 42);
    assert_eq!(ranges(), 42);
    slice_and_empty()
}
