fn longer<'a>(a: &'a Vec<i64>, b: &'a Vec<i64>) -> &'a Vec<i64> { if a.len() >= b.len() { a } else { b } }
pub fn probe() -> i64 { let mut a: Vec<i64> = Vec::new(); a.push(42); let b: Vec<i64> = Vec::new(); longer(&a, &b)[0] }
