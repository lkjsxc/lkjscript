fn first(v: &mut Vec<i64>) -> &mut i64 { &mut v[0] }
pub fn probe() -> i64 {
    let mut v: Vec<i64> = Vec::new(); v.push(1);
    let a = first(&mut v);
    let b = first(&mut v);
    *a = 5;
    *b
}
