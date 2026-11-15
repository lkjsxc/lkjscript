struct H<'a> { r: &'a i64 }
pub fn probe() -> i64 {
    let b: Box<i64> = Box::new(5);
    let h = H { r: &*b };
    let c = b;
    let n: i64 = *h.r;
    n + *c
}
