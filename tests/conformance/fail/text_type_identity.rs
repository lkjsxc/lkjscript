fn text_only(value: &str) -> i64 { 7 }
pub fn probe() -> i64 {
    let value = 1i64;
    text_only(&value)
}
