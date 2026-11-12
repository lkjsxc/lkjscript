pub fn probe() -> i64 {
    let x: Box<i64> = Box::new(1i64);
    let mut i: usize = 0usize;
    while i < 3usize { i = i + 1usize; if i == 2usize { drop(x); break; } }
    *x
}
