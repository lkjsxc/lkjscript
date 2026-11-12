pub fn probe() -> i64 {
    let mut x: Box<i64> = Box::new(1i64);
    drop(x);
    if true { x = Box::new(2i64); }
    *x
}
