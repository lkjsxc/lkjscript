pub fn probe() -> i64 {
    let value: i64 = 42;
    let read = || value;
    read()
}
