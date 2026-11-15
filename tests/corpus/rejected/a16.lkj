fn get<'a>(v: &'a Vec<Box<i64>>) -> &'a i64 { &*v[0] }
pub fn probe() -> i64 {
    let mut v: Vec<Box<i64>> = Vec::new(); v.push(Box::new(1));
    let r: &i64 = get(&v);
    let w = v;
    drop(w);
    *r
}
