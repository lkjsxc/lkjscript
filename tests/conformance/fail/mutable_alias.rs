pub fn probe() -> i64 {
    let mut value: i64 = 0;
    let first = &mut value;
    let second = &mut value;
    *first = 20;
    *second = 22;
    *first + *second
}
