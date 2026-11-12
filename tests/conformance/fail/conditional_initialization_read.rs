pub fn probe() -> i64 {
    let x: Box<i64>;
    if true { x = Box::new(1i64); }
    *x
}
