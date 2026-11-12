pub fn probe() -> i64 {
    let x: Box<i64> = Box::new(1i64);
    if true { drop(x); } else { let r: &Box<i64> = &x; }
    let r: &Box<i64> = &x;
    **r
}
