enum E { A(i64), B(Box<i64>) }
pub fn probe() -> i64 {
    let mut e = E::B(Box::new(7));
    let r: &i64 = match &e { E::B(b) => &**b, E::A(x) => x };
    e = E::A(0);
    *r
}
