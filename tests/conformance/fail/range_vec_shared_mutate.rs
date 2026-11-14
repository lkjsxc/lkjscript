// expect: reject E_BORROW
// witness: {"rustCode":"E0502"}
pub fn probe() -> i64 {
    let mut values = vec![1i64, 2, 3];
    let result = &values[{
        values.push(4);
        0
    }..];
    result[0] + 41
}
