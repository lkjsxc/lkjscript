// expect: reject E_TYPE
// witness: {"rustCode":"E0571"}
pub fn probe() -> i64 {
    let values = vec![42i64];
    for value in &values {
        break *value;
    }
    0
}
