fn escaped() -> impl Fn() -> i64 {
    let value: i64 = 42;
    || value
}
pub fn probe() -> i64 { escaped()() }
