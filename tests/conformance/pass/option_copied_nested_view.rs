fn copied<'a>(value: &'a i64) -> &'a i64 { let temporary = value; let result = Some(&temporary).copied().unwrap(); result }
fn cloned<'a>(value: &'a i64) -> &'a i64 { let temporary = value; let result = Some(&temporary).cloned().unwrap(); result }
pub fn probe() -> i64 { let left = 20i64; let right = 22i64; *copied(&left) + *cloned(&right) }
