fn stored<'a>(carrier: &&'a i64) -> &'a i64 { *carrier }
fn through_carrier<'a>(value: &'a i64) -> &'a i64 {
    let carrier = value;
    stored(&carrier)
}
pub fn probe() -> i64 {
    let value: i64 = 42;
    *through_carrier(&value)
}
