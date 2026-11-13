pub fn probe() -> i64 {
    let mut values = vec![1i64, 2, 3];
    let empty = &mut values[2..2];
    values[0] = 40;
    drop(empty);
    values[0]
}
