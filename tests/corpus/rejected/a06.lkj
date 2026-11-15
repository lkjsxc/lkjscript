pub fn probe() -> i64 {
    let mut v: Vec<i64> = Vec::new(); v.push(1);
    let r = &v;
    let old = std::mem::replace(&mut v, Vec::new());
    drop(old);
    r[0]
}
