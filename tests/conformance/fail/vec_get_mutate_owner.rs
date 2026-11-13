pub fn probe() -> i64 {
    let mut values = vec![Box::new(42i64)];
    let item = values.get(0usize).unwrap();
    values.push(Box::new(1i64));
    **item
}
