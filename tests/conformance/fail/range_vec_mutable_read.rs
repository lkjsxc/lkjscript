pub fn probe() -> i64 {
    let mut values = vec![1i64, 2, 3];
    let result = &mut values[..values.len()];
    result[0] = 40;
    result[0] + result[1]
}
