pub fn probe() -> i64 {
    let mut values: Vec<usize> = Vec::new();
    values.push(10);
    values.push(values.len());
    (values[0] + values[1]) as i64
}
