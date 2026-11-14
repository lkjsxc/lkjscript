// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0593"}
pub fn probe() -> i64 {
    let absent: Option<i64> = None;
    let result = absent.ok_or_else(|value: i64| value);
    result.unwrap_or(42)
}
