// expect: reject E_TRAIT
// witness: {"rustCode":"E0277"}
pub fn probe() -> i64 {
    let values = vec![42i64];
    let _ = values.iter().zip(42i64);
    42
}
