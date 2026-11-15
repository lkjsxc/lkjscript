struct P { a: Box<i64>, b: Box<i64> }
pub fn probe() -> i64 {
    let p = P { a: Box::new(1), b: Box::new(2) };
    let r: &i64 = &*p.a;
    let q = p.a;
    *r + *q
}
