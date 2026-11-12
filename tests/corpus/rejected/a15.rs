pub fn probe() -> i64 {
    let mut v: Vec<Box<i64>> = Vec::new(); v.push(Box::new(1));
    let r: &i64 = &*v[0];
    v[0] = Box::new(2);
    *r
}
