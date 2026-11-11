fn leak() -> &'static str {
    let owned = "local".to_string();
    owned.as_str()
}
pub fn probe() -> i64 {
    leak().len() as i64
}
