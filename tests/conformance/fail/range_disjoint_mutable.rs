pub fn probe() -> i64 {
    let mut values = vec![1i64, 2, 3, 4];
    let first = &mut values[..2];
    let second = &mut values[2..];
    first[0] = 40;
    second[0] = 2;
    first[0] + second[0]
}
