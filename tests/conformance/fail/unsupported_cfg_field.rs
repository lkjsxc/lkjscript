struct Record {
    #[cfg(test)]
    test_only: i64,
    value: i64,
}
pub fn probe() -> i64 { Record { value: 42 }.value }
