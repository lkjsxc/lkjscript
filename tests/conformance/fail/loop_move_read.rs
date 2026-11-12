pub fn probe() -> i64 {
    let x: Box<i64> = Box::new(1i64);
    let mut i: usize = 0usize;
    while i < 2usize { drop(x); i = i + 1usize; }
    0i64
}
