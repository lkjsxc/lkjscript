pub fn probe() -> i64 {
    let mut values = [1i64, 2, 3];
    let slice: &mut [i64] = &mut values;
    let result = &slice[{
        slice[0] = 2;
        0
    }..];
    result[0] + 40
}
