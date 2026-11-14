// expect: reject E_TYPE
// witness: {"rustAccept":false}
pub fn probe() -> i64 {
    let value: u8 = 10;
    match value {
        0..=256 => 1,
        _ => 0,
    }
}
