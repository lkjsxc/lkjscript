pub fn probe() -> i64 {
    let mut value: i64 = 42;
    let borrowed = &value;
    value = 0;
    *borrowed
}
