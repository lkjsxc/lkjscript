pub fn probe() -> i64 {
    let mut x: i64 = 1;
    let a = &mut x;
    let b = &mut *a;
    *a = 2;
    *b = 3;
    x
}
