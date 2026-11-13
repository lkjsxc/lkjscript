pub fn probe() -> i64 {
    let mut values = [1i64, 2, 3];
    let result = &values[{
        values[0] = 2;
        0
    }..];
    result[0] + 40
}
