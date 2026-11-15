// expect: reject E_TYPE
// witness: {"rustCode":"E0271"}
pub fn probe() -> i64 {
    let left = vec![42i64];
    let right = vec![true];
    let _ = left.iter().chain(right.iter());
    42
}
