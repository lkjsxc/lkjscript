fn copied<T: Clone>(value: &T) -> T { value.clone() }
pub fn probe() -> i64 {
    let mut values: Vec<String> = Vec::new();
    values.push("hi".to_string());
    let duplicate = copied(&values);
    assert!(values == duplicate);
    values[0].push_str("!");
    assert!(values != duplicate);
    drop(values);
    duplicate[0].len() as i64 + 40
}
