pub fn probe() -> i64 {
    let capture = Box::new(42i64);
    let predicate = move |value: i64| { drop(capture); value == 42 };
    let first = Some(42i64).is_some_and(predicate);
    let second = Some(42i64).is_none_or(predicate);
    if first && second { 42 } else { 0 }
}
