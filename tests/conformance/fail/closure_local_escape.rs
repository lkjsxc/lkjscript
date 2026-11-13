pub fn probe() -> i64 {
    let read = { let local: i64 = 42; || local };
    read()
}
