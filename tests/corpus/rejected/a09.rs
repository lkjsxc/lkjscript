pub fn probe() -> i64 {
    let mut v: Vec<i64> = Vec::new(); v.push(1);
    let s: &[i64] = &v;
    v.push(2);
    s[0]
}
