mod uncertain {
    use absent::*;
    pub fn unavailable() -> i64 { missing() }
}
fn independent(value: i64) -> i64 { value.unrelated() }
