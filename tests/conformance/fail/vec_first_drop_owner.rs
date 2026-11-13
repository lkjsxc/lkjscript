pub fn probe() -> i64 {
    let values = vec![Box::new(42i64)];
    let item = values.first().unwrap();
    drop(values);
    **item
}
