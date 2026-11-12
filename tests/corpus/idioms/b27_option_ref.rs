fn find(v: &Vec<i64>, k: i64) -> Option<&i64> { let mut i: usize = 0; while i < v.len() { if v[i] == k { return Some(&v[i]); } i += 1; } None }
pub fn probe() -> i64 { let v = vec![1i64, 42]; match find(&v, 42) { Some(r) => *r, None => 0 } }
